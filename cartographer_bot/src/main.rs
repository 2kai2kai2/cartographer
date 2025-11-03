use anyhow::Context;
use lazy_static::lazy_static;
use reservations::{Reservation, ReservationsData};
use serenity::all::{ActivityData, CacheHttp, Ready};
use serenity::async_trait;
use serenity::model::application::*;
use serenity::{
    all::{EventHandler, GatewayIntents},
    builder::*,
    Client,
};
use sqlx::PgPool;
use stats_core::PreprocessedSaveGame;
use std::collections::HashMap;
use std::io::Cursor;

mod db_types;
mod reservations;

#[derive(serde::Deserialize, Debug)]
struct Env {
    discord_token: String,
    postgres_string: String,
}

const PNG_MAP_1444: &[u8] = include_bytes!("../assets/eu4/vanilla/1444.png");
const PNG_ICON_X: &[u8] = include_bytes!("../assets/eu4/xIcon.png");

lazy_static! {
    pub static ref TAGS: HashMap<String, Vec<String>> = {
        let tags = include_str!("../../cartographer_web/resources/eu4/vanilla/tags.txt");
        tags.lines()
            .map(|line| {
                let mut it = line.split(';');
                let tag = it.next().expect("Invalid tags file");
                return (tag.to_string(), it.map(str::to_string).collect());
            })
            .collect()
    };
    pub static ref CAPITAL_LOCATIONS: HashMap<String, (f64, f64)> = {
        let locations = include_str!("../assets/eu4/vanilla/capitals.txt");
        locations
            .lines()
            .map(|line| {
                let mut it = line.split(';');
                let tag = it.next().expect("Missing tag on line");
                let x = it.next().expect("Missing x on line");
                let y = it.next().expect("Missing y on line");
                if tag.len() != 3 || !tag.chars().all(|c| c.is_ascii_uppercase()) {
                    panic!("Invalid tag '{tag}'");
                }
                let x: f64 = x.parse().expect("Failed to parse x location");
                let y: f64 = y.parse().expect("Failed to parse y location");
                (tag.to_string(), (x, y))
            })
            .collect()
    };
}

/// Gets the tag for a country name or tag.
fn get_tag(country: &str) -> Option<String> {
    let country = country.trim();
    if country.len() == 3 && TAGS.contains_key(&country.to_uppercase()) {
        return Some(country.to_uppercase());
    }
    return TAGS.iter().find_map(|(tag, names)| {
        names
            .iter()
            .find(|name| name.eq_ignore_ascii_case(country))
            .and(Some(tag.to_uppercase()))
    });
}

fn make_error_msg(text: impl Into<String>) -> CreateInteractionResponse {
    return CreateInteractionResponse::Message(
        CreateInteractionResponseMessage::new()
            .content(text)
            .ephemeral(true),
    );
}

struct LocalFetcher;
impl LocalFetcher {
    // const LOCAL_PATH: &'static str = "./cartographer_web/resources";
    const LOCAL_PATH: &'static str = "/app/resources";
}
impl stats_core::Fetcher for LocalFetcher {
    async fn get(&self, path: &str) -> anyhow::Result<Vec<u8>> {
        let path = std::path::PathBuf::from(LocalFetcher::LOCAL_PATH).join(path);
        return std::fs::read(path).context("While trying to read file.");
    }
}

struct Handler {
    db: PgPool,
}
/// For reservations
impl Handler {
    async fn handle_reservations_command(
        &self,
        interaction: &CommandInteraction,
    ) -> Result<CreateInteractionResponse, Option<String>> {
        println!("Handling /reservations");
        // TODO: check permissions
        let query = sqlx::query_scalar(
            "
            INSERT INTO games(server_id)
            VALUES($1)
            RETURNING game_id
            ",
        )
        .bind(interaction.guild_id.map(|id| id.get() as i64));
        let game_id: i64 = query
            .fetch_one(&self.db)
            .await
            .map_err(|err| Some(err.to_string()))?;
        let game_id = game_id as u64;
        println!("Gameid {game_id}");

        let reserve_input = CreateButton::new(format!("reserve:{game_id}")).label("Reserve");
        let unreserve_button = CreateButton::new(format!("unreserve:{game_id}"))
            .style(ButtonStyle::Danger)
            .label("Unreserve");
        let action_row = vec![CreateActionRow::Buttons(vec![
            reserve_input,
            unreserve_button,
        ])];

        let reservations = ReservationsData::new();
        let msg = CreateInteractionResponseMessage::new()
            .content(reservations.to_string())
            .components(action_row);
        let msg = match reservations.make_map_png() {
            Ok(img) => msg.files([CreateAttachment::bytes(img, "reservation_map.png")]),
            Err(err) => {
                println!("{err}");
                msg.files([])
            }
        };
        return Ok(CreateInteractionResponse::Message(msg));
    }

    async fn handle_reserve_button(
        &self,
        interaction: &ComponentInteraction,
        game_id: u64,
    ) -> Result<CreateInteractionResponse, Option<String>> {
        // temp: add server id to games since we currently don't have them
        let query = sqlx::query(
            "
            UPDATE games
            SET server_id = $1
            WHERE game_id = $2 AND server_id IS NULL
        ",
        )
        .bind(interaction.guild_id.map(|id| id.get() as i64))
        .bind(game_id as i64);
        query
            .execute(&self.db)
            .await
            .map_err(|err| Some(err.to_string()))?;

        let tag_input = CreateInputText::new(InputTextStyle::Short, "EU4 Country Tag", "tag")
            .placeholder("Name (Sweden) or tag (SWE)");
        let modal = CreateModal::new(format!("reserve:{game_id}"), "Select country tag")
            .components(vec![CreateActionRow::InputText(tag_input)]);
        return Ok(CreateInteractionResponse::Modal(modal));
    }

    async fn handle_unreserve_interaction(
        &self,
        interaction: &ComponentInteraction,
        game_id: u64,
    ) -> Result<CreateInteractionResponse, Option<String>> {
        let delete_query = sqlx::query(
            "
            DELETE FROM reservations
            WHERE game_id = $1 AND user_id = $2
            ",
        )
        .bind(game_id as i64)
        .bind(interaction.user.id.get() as i64);

        let items_query = sqlx::query_as::<_, db_types::RawReservation>(
            "
            SELECT user_id, timestamp, tag
            FROM reservations
            WHERE game_id = $1
            ORDER BY timestamp ASC
            ",
        )
        .bind(game_id as i64);
        let mut tr = self.db.begin().await.or(Err(None))?;
        delete_query.execute(&mut *tr).await.or(Err(None))?;
        let reservations = items_query.fetch_all(&mut *tr).await.or(Err(None))?;
        tr.commit().await.or(Err(None))?;
        println!("queries done");

        let reservations = reservations.into_iter().map(Reservation::from).collect();
        let reservations = ReservationsData { reservations };
        let msg = CreateInteractionResponseMessage::new().content(reservations.to_string());
        let msg = match reservations.make_map_png() {
            Ok(img) => msg.files([CreateAttachment::bytes(img, "reservation_map.png")]),
            Err(err) => {
                println!("{err}");
                msg.files([])
            }
        };
        return Ok(CreateInteractionResponse::UpdateMessage(msg));
    }

    async fn handle_reserve_modal(
        &self,
        interaction: &ModalInteraction,
        country: &String,
        game_id: u64,
    ) -> Result<CreateInteractionResponse, Option<String>> {
        let tag = get_tag(&country).ok_or(Some("Unrecognized country name or tag.".to_string()))?;

        let check_query = sqlx::query_scalar::<_, bool>(
            "
            SELECT EXISTS(
                SELECT 1
                FROM reservations
                WHERE game_id = $1
                AND tag = $2
            )
            ",
        )
        .bind(game_id as i64)
        .bind(&tag);

        let insert_query = sqlx::query(
            "
            INSERT INTO reservations (
                game_id,
                user_id,
                timestamp,
                tag
            )
            VALUES (
                $1,
                $2,
                $3,
                $4
            )
            ON CONFLICT (game_id, user_id) DO UPDATE SET
                timestamp = excluded.timestamp,
                tag = excluded.tag
            ",
        )
        .bind(game_id as i64)
        .bind(interaction.user.id.get() as i64)
        .bind(chrono::offset::Utc::now())
        .bind(&tag);

        let items_query = sqlx::query_as::<_, db_types::RawReservation>(
            "
            SELECT user_id, timestamp, tag
            FROM reservations
            WHERE game_id = $1
            ORDER BY timestamp ASC
            ",
        )
        .bind(game_id as i64);

        let mut tr = self
            .db
            .begin()
            .await
            .map_err(|err| format!("ERROR: while initiating transaction: {err}"))?;
        match check_query.fetch_one(&mut *tr).await {
            Err(err) => return Err(Some(format!("ERROR: while checking tag: {err}"))),
            Ok(true) => return Err(Some(format!("The tag {tag} is already reserved."))),
            Ok(false) => (),
        };
        insert_query
            .execute(&mut *tr)
            .await
            .map_err(|err| format!("ERROR: while inserting: {err}"))?;
        let reservations = items_query
            .fetch_all(&mut *tr)
            .await
            .map_err(|err| format!("ERROR: while fetching new state: {err}"))?;
        tr.commit()
            .await
            .map_err(|err| format!("ERROR: while committing transaction: {err}"))?;

        let reservations = reservations.into_iter().map(Reservation::from).collect();
        let reservations = ReservationsData { reservations };
        let msg = CreateInteractionResponseMessage::new().content(reservations.to_string());
        let msg = match reservations.make_map_png() {
            Ok(img) => msg.files([CreateAttachment::bytes(img, "reservation_map.png")]),
            Err(err) => {
                println!("{err}");
                msg.files([])
            }
        };
        return Ok(CreateInteractionResponse::UpdateMessage(msg));
    }
}

/// For stats
impl Handler {
    async fn handle_stats_command(
        &self,
        ctx: &serenity::client::Context,
        interaction: &CommandInteraction,
    ) -> Result<CreateInteractionResponse, Option<String>> {
        let options = interaction.data.options();
        let save_file = options
            .iter()
            .find(|option| option.name == "save_file")
            .ok_or(Some("A save file must be specified.".to_string()))?;
        let ResolvedValue::Attachment(attachment) = save_file.value else {
            return Err(Some(
                "Unexpected option value type. `save_file` should be an attachment.".to_string(),
            ));
        };

        let _ = interaction
            .create_response(
                ctx.http(),
                CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
            )
            .await;

        let time_download_start = std::time::Instant::now();
        let save_file_name = attachment.filename.clone();
        let raw_save = attachment
            .download()
            .await
            .map_err(|err| Some(err.to_string()))?;

        let time_parse_start = std::time::Instant::now();
        let preprocessed_save = PreprocessedSaveGame::preprocess(&raw_save, &save_file_name)
            .map_err(|err| Some(err.to_string()))?;
        drop(raw_save);
        let parsed_save = preprocessed_save
            .to_parsed()
            .map_err(|err| Some(err.to_string()))?;

        let time_render_start = std::time::Instant::now();
        let game_id = parsed_save.id();
        let img = match parsed_save {
            stats_core::SomeSaveGame::EU4(save_game) => {
                stats_core::eu4::render_stats_image(&LocalFetcher, save_game).await
            }
            stats_core::SomeSaveGame::Stellaris(save_game) => {
                stats_core::stellaris::render_stats_image_stellaris(&LocalFetcher, save_game).await
            }
        }
        .map_err(|err| Some(err.to_string()))?;

        let time_upload_start = std::time::Instant::now();
        let mut png_buffer: Vec<u8> = Vec::new();
        img.write_to(&mut Cursor::new(&mut png_buffer), image::ImageFormat::Png)
            .map_err(|_| Some("Failed to save image to PNG.".to_string()))?;

        let _ = interaction
            .edit_response(
                ctx.http(),
                EditInteractionResponse::new()
                    .new_attachment(CreateAttachment::bytes(png_buffer, "image.png")),
            )
            .await;

        let time_done = std::time::Instant::now();
        println!(
            "Stats for {game_id:9} | download {:5.2}s | parse {:5.2}s | render {:5.2}s | upload {:5.2}s",
            time_parse_start
                .duration_since(time_download_start)
                .as_secs_f32(),
            time_render_start
                .duration_since(time_parse_start)
                .as_secs_f32(),
            time_upload_start
                .duration_since(time_render_start)
                .as_secs_f32(),
            time_done.duration_since(time_upload_start).as_secs_f32()
        );

        return Err(None); // we handle responses internally since we have updating messages
    }
}

impl Handler {
    async fn handle_command_interaction(
        &self,
        ctx: &serenity::client::Context,
        interaction: &CommandInteraction,
    ) -> Result<CreateInteractionResponse, Option<String>> {
        match interaction.data.name.as_str() {
            "reservations" => self.handle_reservations_command(interaction).await,
            "stats" => self.handle_stats_command(ctx, interaction).await,
            _ => Err(Some("Unsupported command".to_string())),
        }
    }

    async fn handle_component_interaction(
        &self,
        ctx: &serenity::client::Context,
        interaction: &ComponentInteraction,
    ) -> Result<CreateInteractionResponse, Option<String>> {
        return match (
            &interaction.data.kind,
            interaction.data.custom_id.split_once(':'),
        ) {
            (ComponentInteractionDataKind::Button, Some(("reserve", game_id))) => {
                let Ok(game_id) = game_id.parse::<u64>() else {
                    return Err(Some("ERROR: failed to parse game id".to_string()));
                };
                self.handle_reserve_button(interaction, game_id).await
            }
            (ComponentInteractionDataKind::Button, Some(("unreserve", game_id))) => {
                let Ok(game_id) = game_id.parse::<u64>() else {
                    return Err(Some("ERROR: failed to parse game id".to_string()));
                };
                self.handle_unreserve_interaction(interaction, game_id)
                    .await
            }
            _ => Err(None),
        };
    }

    async fn handle_modal_interaction(
        &self,
        ctx: &serenity::client::Context,
        interaction: &ModalInteraction,
    ) -> Result<CreateInteractionResponse, Option<String>> {
        return match interaction.data.custom_id.split_once(':') {
            Some(("reserve", game_id)) => {
                let Some(row) = interaction.data.components.get(0) else {
                    return Err(Some("Missing action row".to_string()));
                };
                let [ActionRowComponent::InputText(input_text)] = row.components.as_slice() else {
                    return Err(Some("Incorrect modal contents".to_string()));
                };
                let Ok(game_id) = game_id.parse::<u64>() else {
                    return Err(Some("ERROR: failed to parse game id".to_string()));
                };
                let Some(country) = &input_text.value else {
                    return Err(None);
                };
                return self
                    .handle_reserve_modal(interaction, country, game_id)
                    .await;
            }
            _ => Err(None),
        };
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: serenity::client::Context, interaction: Interaction) {
        let _ = match &interaction {
            Interaction::Command(command) => {
                match self.handle_command_interaction(&ctx, command).await {
                    Ok(response) => command
                        .create_response(ctx.http, response)
                        .await
                        .inspect_err(|msg| println!("ERROR: {msg}")),
                    Err(Some(msg)) => command.create_response(ctx.http, make_error_msg(msg)).await,
                    Err(None) => Ok(()),
                }
            }
            Interaction::Component(interaction) => {
                match self.handle_component_interaction(&ctx, interaction).await {
                    Ok(response) => interaction.create_response(ctx.http, response).await,
                    Err(Some(msg)) => {
                        println!("An error occurred during an interaction: {msg}");
                        interaction
                            .create_response(ctx.http, make_error_msg(msg))
                            .await
                    }
                    Err(None) => Ok(()),
                }
            }
            Interaction::Modal(interaction) => {
                match self.handle_modal_interaction(&ctx, interaction).await {
                    Ok(response) => interaction.create_response(ctx.http, response).await,
                    Err(Some(msg)) => {
                        println!("An error occurred during a modal interaction: {msg}");
                        interaction
                            .create_response(ctx.http, make_error_msg(msg))
                            .await
                    }
                    Err(None) => Ok(()),
                }
            }
            _ => return,
        };
    }
    async fn ready(&self, ctx: serenity::client::Context, ready: Ready) {
        register_commands(&ctx.http).await;
        println!("Ready!");
    }
}

async fn register_commands(http: &impl serenity::http::CacheHttp) {
    let reservations_command = CreateCommand::new("reservations")
        .description("Creates a new Cartographer EU4 reservations message.");
    let _ = Command::create_global_command(&http, reservations_command)
        .await
        .unwrap();
    let stats_command = CreateCommand::new("stats")
        .description("Creates a stats image from a save file.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Attachment,
                "save_file",
                "The save file to use",
            )
            .required(true),
        );
    let _ = Command::create_global_command(&http, stats_command)
        .await
        .unwrap();
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    if let Ok(_) = dotenvy::dotenv() {
        println!("Loaded .env");
    }
    let env: Env = envy::from_env()?;
    let db = PgPool::connect_lazy(&env.postgres_string)?;
    let mut client = Client::builder(&env.discord_token, GatewayIntents::empty())
        .event_handler(Handler { db })
        .activity(ActivityData::custom("Taking EU4 Reservations"))
        .await
        .context("While creating client.")?;

    return client.start().await.context("While running the client.");
}
