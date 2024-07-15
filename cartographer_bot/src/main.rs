use anyhow::Context;
use lazy_static::lazy_static;
use reservations::{Reservation, ReservationsData};
use serenity::all::{ActivityData, Ready};
use serenity::model::application::*;
use serenity::{
    all::{EventHandler, GatewayIntents},
    builder::*,
    Client,
};
use serenity::{async_trait, json};
use shuttle_runtime::SecretStore;
use sqlx::PgPool;
use std::collections::HashMap;

mod db_types;
mod reservations;

lazy_static! {
    static ref TAGS: HashMap<String, Vec<String>> = {
        let tags = include_str!("../../cartographer_web/resources/vanilla/tags.txt");
        tags.lines()
            .map(|line| {
                let mut it = line.split(';');
                let tag = it.next().expect("Invalid tags file");
                return (tag.to_string(), it.map(str::to_string).collect());
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

struct Handler {
    db: PgPool,
}
impl Handler {
    async fn reservations_command(&self) -> Result<CreateInteractionResponse, Option<String>> {
        println!("Handling /reservations");
        // TODO: check permissions
        let game_id: i64 = sqlx::query_scalar("INSERT INTO Games DEFAULT VALUES RETURNING game_id")
            .fetch_one(&self.db)
            .await
            .map_err(|err| Some(err.to_string()))?;
        let game_id = game_id as u64;
        println!("Gameid {game_id}");

        let res = ReservationsData::new();
        let reserve_input = CreateButton::new(format!("reserve:{game_id}")).label("Reserve");
        let unreserve_button = CreateButton::new(format!("unreserve:{game_id}"))
            .style(ButtonStyle::Danger)
            .label("Unreserve");
        let action_row = vec![CreateActionRow::Buttons(vec![
            reserve_input,
            unreserve_button,
        ])];
        let response = CreateInteractionResponseMessage::new()
            .content(format!("{res}"))
            .components(action_row);
        println!(
            "Made response {}",
            json::to_string(&response).unwrap_or("ERR".to_string())
        );
        return Ok(CreateInteractionResponse::Message(response));
    }

    async fn handle_reserve_button(
        &self,
        interaction: &ComponentInteraction,
        game_id: u64,
    ) -> Result<CreateInteractionResponse, Option<String>> {
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
            DELETE FROM Reservations
            WHERE game_id = $1 AND user_id = $2
            ",
        )
        .bind(game_id as i64)
        .bind(interaction.user.id.get() as i64);

        let items_query = sqlx::query_as::<_, db_types::RawReservation>(
            "
            SELECT user_id, timestamp, tag
            FROM Reservations
            WHERE game_id = $1
            ORDER BY timestamp ASC
            ",
        )
        .bind(game_id as i64);

        let mut tr = self.db.begin().await.or(Err(None))?;
        delete_query.execute(&mut *tr).await.or(Err(None))?;
        let reservations = items_query.fetch_all(&mut *tr).await.or(Err(None))?;
        tr.commit().await.or(Err(None))?;

        let reservations = reservations.into_iter().map(Reservation::from).collect();
        let msg = CreateInteractionResponseMessage::new()
            .content(format!("{}", ReservationsData { reservations }));
        return Ok(CreateInteractionResponse::UpdateMessage(msg));
    }

    async fn handle_command_interaction(
        &self,
        ctx: &serenity::client::Context,
        interaction: &CommandInteraction,
    ) -> Result<CreateInteractionResponse, Option<String>> {
        match interaction.data.name.as_str() {
            "reservations" => self.reservations_command().await,
            "stats" => Ok(CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content(
                        "\
                        Stats functionality is currently moved to https://2kai2kai2.github.io/cartographer/
                        You can upload any non-ironman save to generate an image you can upload to Discord.\
                        ",
                    )
                    .ephemeral(true),
            )),
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
            INSERT INTO Reservations (
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
            FROM Reservations
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
        let msg = CreateInteractionResponseMessage::new()
            .content(format!("{}", ReservationsData { reservations }));
        return Ok(CreateInteractionResponse::UpdateMessage(msg));
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
        println!("Ready!");
    }
}

#[shuttle_runtime::main]
async fn serenity(
    #[shuttle_runtime::Secrets] secrets: SecretStore,
    #[shuttle_shared_db::Postgres] pool: PgPool,
) -> shuttle_serenity::ShuttleSerenity {
    let token = secrets
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    let client = Client::builder(&token, GatewayIntents::empty())
        .event_handler(Handler { db: pool })
        .activity(ActivityData::custom("Taking EU4 Reservations"))
        .await
        .context("Err creating client")?;

    return Ok(client.into());
}
