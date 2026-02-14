use anyhow::Context;
use reservations::{Reservation, ReservationsData};
use serenity::all::{ActivityData, CacheHttp, Ready};
use serenity::async_trait;
use serenity::model::application::*;
use serenity::{
    Client,
    all::{EventHandler, GatewayIntents},
    builder::*,
};
use sqlx::PgPool;
use stats_core::{EU4ParserStepText, GameSaveType, StellarisParserStepText};
use std::io::Cursor;
use std::str::FromStr;

use crate::assets::Tags;
use crate::reservations::GameType;

mod assets;
mod db_types;
mod reservations;

const ASSETS_BASE_PATH: &str = "/app/assets";
lazy_static::lazy_static! {
    static ref TAGS_EU4_VANILLA: Tags = Tags::parse_new(std::fs::read_to_string(format!("{ASSETS_BASE_PATH}/eu4/vanilla/tags.txt")).unwrap()).unwrap();
    static ref TAGS_EU5_VANILLA: Tags = Tags::parse_new(std::fs::read_to_string(format!("{ASSETS_BASE_PATH}/eu5/vanilla/tags.txt")).unwrap()).unwrap();
}
pub fn get_tags(game_type: GameType) -> &'static Tags {
    return match game_type {
        GameType::EU4 => &TAGS_EU4_VANILLA,
        GameType::EU5 => &TAGS_EU5_VANILLA,
    };
}

#[derive(serde::Deserialize, Debug)]
struct Env {
    discord_token: String,
    postgres_string: String,
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
        ctx: &serenity::client::Context,
        interaction: &CommandInteraction,
    ) -> Result<(), String> {
        println!("Handling /reservations");
        let game_type = interaction
            .data
            .options
            .iter()
            .find(|option| option.name == "game")
            .ok_or_else(|| "Missing game option".to_string())?;
        let game_type = match &game_type.value {
            CommandDataOptionValue::String(game_type) => game_type,
            _ => return Err("Invalid game option".to_string()),
        };
        let game_type = GameType::from_str(game_type)
            .map_err(|_| format!("Unknown game name '{game_type}'"))?;

        // TODO: check permissions
        let query = sqlx::query_scalar(
            "
            INSERT INTO games(server_id, game_type)
            VALUES($1, $2)
            RETURNING game_id
            ",
        )
        .bind(interaction.guild_id.map(|id| id.get() as i64))
        .bind(game_type);
        let game_id: i64 = query
            .fetch_one(&self.db)
            .await
            .map_err(|err| err.to_string())?;
        let game_id = game_id as u64;

        let reserve_input = CreateButton::new(format!("reserve:{game_id}")).label("Reserve");
        let unreserve_button = CreateButton::new(format!("unreserve:{game_id}"))
            .style(ButtonStyle::Danger)
            .label("Unreserve");
        let action_row = vec![CreateActionRow::Buttons(vec![
            reserve_input,
            unreserve_button,
        ])];

        let reservations = ReservationsData::new(game_type);

        let tags = std::fs::read_to_string(game_type.get_tags_path(ASSETS_BASE_PATH))
            .map_err(|err| err.to_string())?;
        let tags = Tags::parse_new(tags).map_err(|err| err.to_string())?;
        let msg =
            std::fmt::from_fn(|f| reservations.format_with_game(game_type, &tags, f)).to_string();
        let msg = CreateInteractionResponseMessage::new()
            .content(msg)
            .components(action_row);
        let msg = match reservations.make_map_png().await {
            Ok(img) => msg.files([CreateAttachment::bytes(img, "reservation_map.png")]),
            Err(err) => {
                println!("{err}");
                msg.files([])
            }
        };

        interaction
            .create_response(&ctx.http, CreateInteractionResponse::Message(msg))
            .await
            .map_err(|err| err.to_string())
    }

    async fn handle_reserve_button(
        &self,
        ctx: &serenity::client::Context,
        interaction: &ComponentInteraction,
        game_id: u64,
    ) -> Result<(), String> {
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
        let db = self.db.clone();
        tokio::spawn(async move {
            let _ = query.execute(&db).await;
        });

        let tag_input = CreateInputText::new(InputTextStyle::Short, "Country Tag", "tag")
            .placeholder("Name (Sweden) or tag (SWE)");
        let modal = CreateModal::new(format!("reserve:{game_id}"), "Select country tag")
            .components(vec![CreateActionRow::InputText(tag_input)]);
        interaction
            .create_response(ctx.http(), CreateInteractionResponse::Modal(modal))
            .await
            .map_err(|err| err.to_string())
    }

    async fn handle_unreserve_interaction(
        &self,
        ctx: &serenity::client::Context,
        interaction: &ComponentInteraction,
        game_id: u64,
    ) -> Result<(), String> {
        let delete_query = sqlx::query(
            "
            DELETE FROM reservations
            WHERE game_id = $1 AND user_id = $2
            ",
        )
        .bind(game_id as i64)
        .bind(interaction.user.id.get() as i64);

        let game_query = sqlx::query_scalar(
            "
            SELECT game_type
            FROM games
            WHERE game_id = $1
            ",
        )
        .bind(game_id as i64);
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
            .or(Err("Failed to begin database transaction".to_string()))?;
        delete_query
            .execute(&mut *tr)
            .await
            .or(Err("Failed to delete reservations".to_string()))?;
        let game_type: GameType = game_query
            .fetch_one(&mut *tr)
            .await
            .or(Err("Failed to fetch game type".to_string()))?;
        let reservations = items_query
            .fetch_all(&mut *tr)
            .await
            .or(Err("Failed to fetch new set of reservations".to_string()))?;
        tr.commit()
            .await
            .or(Err("Failed to commit database transaction".to_string()))?;

        let reservations = reservations.into_iter().map(Reservation::from).collect();
        let reservations = ReservationsData {
            reservations,
            game_type,
        };
        let msg =
            std::fmt::from_fn(|f| reservations.format_with_game(game_type, get_tags(game_type), f))
                .to_string();
        let msg = CreateInteractionResponseMessage::new().content(msg);

        let (msg, img) = tokio::join!(
            interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(msg)),
            tokio::spawn(async move { reservations.make_map_png().await })
        );
        msg.map_err(|err| err.to_string())?;

        let attachments = match img {
            Ok(Ok(img)) => {
                EditAttachments::new().add(CreateAttachment::bytes(img, "reservation_map.png"))
            }
            Ok(Err(err)) => {
                println!("{err}");
                EditAttachments::new()
            }
            Err(err) => {
                println!("{err}");
                EditAttachments::new()
            }
        };
        interaction
            .edit_response(
                ctx.http(),
                EditInteractionResponse::new().attachments(attachments),
            )
            .await
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    async fn handle_reserve_modal(
        &self,
        ctx: &serenity::client::Context,
        interaction: &ModalInteraction,
        country: &String,
        game_id: u64,
    ) -> Result<(), String> {
        let game_query = sqlx::query_scalar(
            "
            SELECT game_type
            FROM games
            WHERE game_id = $1
            ",
        )
        .bind(game_id as i64);
        let game_type: GameType = game_query
            .fetch_one(&self.db)
            .await
            .map_err(|err| format!("ERROR: while fetching game type: {err}"))?;

        let tags = get_tags(game_type);
        let tag = tags
            .get_tag_for_name(country)
            .ok_or("Unrecognized country name or tag.".to_string())?;

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
            Err(err) => return Err(format!("ERROR: while checking tag: {err}")),
            Ok(true) => return Err(format!("The tag {tag} is already reserved.")),
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
        let reservations = ReservationsData {
            reservations,
            game_type,
        };
        let msg =
            std::fmt::from_fn(|f| reservations.format_with_game(game_type, get_tags(game_type), f))
                .to_string();
        let msg = CreateInteractionResponseMessage::new().content(msg);

        let (msg, img) = tokio::join!(
            interaction.create_response(ctx.http(), CreateInteractionResponse::UpdateMessage(msg)),
            tokio::spawn(async move { reservations.make_map_png().await })
        );
        msg.map_err(|err| err.to_string())?;

        let attachments = match img {
            Ok(Ok(img)) => {
                EditAttachments::new().add(CreateAttachment::bytes(img, "reservation_map.png"))
            }
            Ok(Err(err)) => {
                println!("{err}");
                EditAttachments::new()
            }
            Err(err) => {
                println!("{err}");
                EditAttachments::new()
            }
        };
        interaction
            .edit_response(
                ctx.http(),
                EditInteractionResponse::new().attachments(attachments),
            )
            .await
            .map_err(|err| err.to_string())?;
        Ok(())
    }
}

/// For stats
impl Handler {
    async fn handle_stats_command(
        &self,
        ctx: &serenity::client::Context,
        interaction: &CommandInteraction,
    ) -> Result<(), String> {
        return Err(
            "This command is currently disabled due to resource constraints.
        Use https://2kai2kai2.github.io/cartographer/ instead."
                .to_string(),
        );

        let options = interaction.data.options();
        let save_file = options
            .iter()
            .find(|option| option.name == "save_file")
            .ok_or("A save file must be specified.".to_string())?;
        let ResolvedValue::Attachment(attachment) = save_file.value else {
            return Err(
                "Unexpected option value type. `save_file` should be an attachment.".to_string(),
            );
        };

        let _ = interaction
            .create_response(
                ctx.http(),
                CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
            )
            .await
            .map_err(|err| err.to_string())?;

        let time_download_start = std::time::Instant::now();
        let save_file_name = attachment.filename.clone();
        let raw_save = attachment.download().await.map_err(|err| err.to_string())?;

        let time_parse_preprocess_start = std::time::Instant::now();
        let game = GameSaveType::determine_from_filename(&save_file_name)
            .ok_or("Could not determine the game type from the filename.".to_string())?;
        let (final_img, mut timings) = match game {
            GameSaveType::EU4 => {
                let mut timings = vec![
                    (time_download_start, "start"),
                    (time_parse_preprocess_start, "downloaded"),
                ];

                let text =
                    EU4ParserStepText::decode_from(&raw_save).map_err(|err| err.to_string())?;
                timings.push((std::time::Instant::now(), "preprocess_done"));

                let raw = text.parse().map_err(|err| err.to_string())?;
                timings.push((std::time::Instant::now(), "parse1_done"));

                let save = raw.parse().map_err(|err| err.to_string())?;
                drop(text);
                timings.push((std::time::Instant::now(), "parse2_done"));

                let img = stats_core::eu4::render_stats_image(&LocalFetcher, save)
                    .await
                    .map_err(|err| err.to_string())?;
                timings.push((std::time::Instant::now(), "render_done"));

                (img, timings)
            }
            GameSaveType::Stellaris => {
                let mut timings = vec![
                    (time_download_start, "start"),
                    (time_parse_preprocess_start, "downloaded"),
                ];

                let text = StellarisParserStepText::decode_from(&raw_save)
                    .map_err(|err| err.to_string())?;
                timings.push((std::time::Instant::now(), "preprocess_done"));

                let raw = text.parse().map_err(|err| err.to_string())?;
                timings.push((std::time::Instant::now(), "parse1_done"));

                let save = raw.parse().map_err(|err| err.to_string())?;
                drop(text);
                timings.push((std::time::Instant::now(), "parse2_done"));

                let img = stats_core::stellaris::render_stats_image_stellaris(&LocalFetcher, save)
                    .await
                    .map_err(|err| err.to_string())?;
                timings.push((std::time::Instant::now(), "render_done"));

                (img, timings)
            }
            GameSaveType::EU5 => {
                return Err("EU5 not yet implemented for bot".to_string());
            }
        };

        let mut png_buffer: Vec<u8> = Vec::new();
        final_img
            .write_to(&mut Cursor::new(&mut png_buffer), image::ImageFormat::Png)
            .map_err(|_| "Failed to save image to PNG.".to_string())?;

        let _ = interaction
            .edit_response(
                ctx.http(),
                EditInteractionResponse::new()
                    .new_attachment(CreateAttachment::bytes(png_buffer, "image.png")),
            )
            .await
            .map_err(|err| format!("ERROR: while editing response: {err}"))?;

        timings.push((std::time::Instant::now(), "upload_done"));
        print!("Stats for {:9}", game.id());
        for pair in timings.windows(2) {
            let &[(start, _), (end, name)] = pair else {
                unreachable!()
            };
            print!(" | {name} {:5.2}s", end.duration_since(start).as_secs_f32(),);
        }
        println!("");
        return Ok(());
    }
}

impl Handler {
    async fn handle_command_interaction(
        &self,
        ctx: &serenity::client::Context,
        interaction: &CommandInteraction,
    ) -> Result<(), String> {
        match interaction.data.name.as_str() {
            "reservations" => self.handle_reservations_command(ctx, interaction).await,
            "stats" => self.handle_stats_command(ctx, interaction).await,
            _ => Err("Unsupported command".to_string()),
        }
    }

    async fn handle_component_interaction(
        &self,
        ctx: &serenity::client::Context,
        interaction: &ComponentInteraction,
    ) -> Result<(), String> {
        return match (
            &interaction.data.kind,
            interaction.data.custom_id.split_once(':'),
        ) {
            (ComponentInteractionDataKind::Button, Some(("reserve", game_id))) => {
                let Ok(game_id) = game_id.parse::<u64>() else {
                    return Err("ERROR: failed to parse game id".to_string());
                };
                self.handle_reserve_button(ctx, interaction, game_id).await
            }
            (ComponentInteractionDataKind::Button, Some(("unreserve", game_id))) => {
                let Ok(game_id) = game_id.parse::<u64>() else {
                    return Err("ERROR: failed to parse game id".to_string());
                };
                self.handle_unreserve_interaction(ctx, interaction, game_id)
                    .await
            }
            _ => Err("Unknown component button identifier".to_string()),
        };
    }

    async fn handle_modal_interaction(
        &self,
        ctx: &serenity::client::Context,
        interaction: &ModalInteraction,
    ) -> Result<(), String> {
        return match interaction.data.custom_id.split_once(':') {
            Some(("reserve", game_id)) => {
                let Some(row) = interaction.data.components.get(0) else {
                    return Err("Missing action row".to_string());
                };
                let [ActionRowComponent::InputText(input_text)] = row.components.as_slice() else {
                    return Err("Incorrect modal contents".to_string());
                };
                let Ok(game_id) = game_id.parse::<u64>() else {
                    return Err("ERROR: failed to parse game id".to_string());
                };
                let Some(country) = &input_text.value else {
                    return Ok(());
                };
                return self
                    .handle_reserve_modal(ctx, interaction, country, game_id)
                    .await;
            }
            _ => Err("Unknown modal button identifier".to_string()),
        };
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn interaction_create(&self, ctx: serenity::client::Context, interaction: Interaction) {
        match &interaction {
            Interaction::Command(command) => {
                match self.handle_command_interaction(&ctx, command).await {
                    Ok(()) => {}
                    Err(msg) => {
                        println!("An error occurred during a command interaction: {msg}");
                        let _ = command.create_response(ctx.http, make_error_msg(msg)).await;
                    }
                }
            }
            Interaction::Component(interaction) => {
                match self.handle_component_interaction(&ctx, interaction).await {
                    Ok(()) => {}
                    Err(msg) => {
                        println!("An error occurred during a component interaction: {msg}");
                        let _ = interaction
                            .create_response(ctx.http, make_error_msg(msg))
                            .await;
                    }
                }
            }
            Interaction::Modal(interaction) => {
                match self.handle_modal_interaction(&ctx, interaction).await {
                    Ok(()) => {}
                    Err(msg) => {
                        println!("An error occurred during a modal interaction: {msg}");
                        let _ = interaction
                            .create_response(ctx.http, make_error_msg(msg))
                            .await;
                    }
                }
            }
            _ => return,
        }
    }
    async fn ready(&self, ctx: serenity::client::Context, ready: Ready) {
        register_commands(&ctx.http).await;
        println!("Ready!");
    }
}

async fn register_commands(http: &impl serenity::http::CacheHttp) {
    let reservations_command = CreateCommand::new("reservations")
        .description("Creates a new Cartographer reservations message.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "game",
                "Which game to create reservations for.",
            )
            .add_string_choice("EU5", "EU5")
            .add_string_choice("EU4", "EU4")
            .required(true),
        );
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
