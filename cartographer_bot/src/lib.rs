use reservations::{Reservation, ReservationsData};
use serde::Deserialize;
use serenity::builder::*;
use serenity::http::Http;
use serenity::interactions_endpoint::Verifier;
use serenity::json::json;
use serenity::model::application::*;
use worker::*;

mod reservations;

/// If successful, it returns the body
async fn check_request(mut req: Request, public_key: &str) -> worker::Result<Vec<u8>> {
    let verifier = Verifier::new(public_key);
    let signature = req
        .headers()
        .get("X-Signature-Ed25519")?
        .ok_or(worker::Error::from("Expected signature key header"))?;
    let timestamp = req
        .headers()
        .get("X-Signature-Timestamp")?
        .ok_or(worker::Error::from("Expected signature timestamp header"))?;

    let body = req.bytes().await?;
    if verifier.verify(&signature, &timestamp, &body).is_err() {
        return Err(worker::Error::from("Body did not match signature"));
    }

    return Ok(body);
}

async fn reservations_command(
    command: CommandInteraction,
    db: &D1Database,
) -> Result<CreateInteractionResponseMessage> {
    // TODO: check permissions
    let game_id: u64 = query!(db, "INSERT INTO Games DEFAULT VALUES RETURNING game_id")
        .first(Some("game_id"))
        .await?
        .ok_or("Insert into Games returned nothing")?;

    let res = ReservationsData::new();
    let reserve_input = CreateInputText::new(
        InputTextStyle::Short,
        "Reserve",
        format!("reserve:{game_id}"),
    );
    let unreserve_button = CreateButton::new(format!("unreserve:{game_id}"))
        .style(ButtonStyle::Danger)
        .label("Unreserve");
    let action_row = vec![
        CreateActionRow::InputText(reserve_input),
        CreateActionRow::Buttons(vec![unreserve_button]),
    ];
    let response = CreateInteractionResponseMessage::new()
        .content(format!("{res}"))
        .components(action_row);
    return Ok(response);
}

async fn handle_reserve_interaction(
    interaction: &ComponentInteraction,
    db: &D1Database,
    tag: &String,
    game_id: u64,
) -> Result<CreateInteractionResponse> {
    let insert_query = query!(
        db,
        "
        INSERT OR REPLACE INTO Reservations (
            game_id,
            user_id,
            timestamp,
            tag
        ) VALUES (
            $1,
            $2,
            $3,
            $4
        )
        ",
        game_id,
        interaction.user.id,
        chrono::offset::Utc::now(),
        tag,
    )?;
    let items_query = query!(
        db,
        "
        SELECT user_id, timestamp, tag
        FROM Reservations
        WHERE game_id = $1
        ORDER BY timestamp ASC
        ",
        game_id
    )?;

    let query_result = db.batch(vec![insert_query, items_query]).await?;
    let query_result = query_result.get(1).ok_or("Failed to get reservations")?;
    let reservations = query_result.results::<Reservation>()?;

    let msg = CreateInteractionResponseMessage::new()
        .content(format!("{}", ReservationsData { reservations }));
    return Ok(CreateInteractionResponse::UpdateMessage(msg));
}

async fn handle_unreserve_interaction(
    interaction: &ComponentInteraction,
    db: &D1Database,
    game_id: u64,
) -> Result<CreateInteractionResponse> {
    let delete_query = query!(
        db,
        "
        DELETE FROM Reservations
        WHERE game_id = $1 AND user_id = $2
        ",
        game_id,
        interaction.user.id,
    )?;
    let items_query = query!(
        db,
        "
        SELECT user_id, timestamp, tag
        FROM Reservations
        WHERE game_id = $1
        ORDER BY timestamp ASC
        ",
        game_id
    )?;
    let query_result = db.batch(vec![delete_query, items_query]).await?;
    let query_result = query_result.get(1).ok_or("Failed to get reservations")?;
    let reservations = query_result.results::<Reservation>()?;

    let msg = CreateInteractionResponseMessage::new()
        .content(format!("{}", ReservationsData { reservations }));
    return Ok(CreateInteractionResponse::UpdateMessage(msg));
}

async fn handle_component_interaction(
    interaction: &ComponentInteraction,
    db: &D1Database,
) -> Result<CreateInteractionResponse> {
    return match (
        &interaction.data.kind,
        interaction.data.custom_id.split_once(':'),
    ) {
        (ComponentInteractionDataKind::StringSelect { values }, Some(("reserve", game_id))) => {
            let [tag] = values.as_slice() else {
                return Err("Only one tag can be selected".into());
            };
            let game_id: u64 = game_id
                .parse()
                .or::<worker::Error>(Err("Failed to parse game id".into()))?;
            handle_reserve_interaction(interaction, db, tag, game_id).await
        }
        (ComponentInteractionDataKind::Button, Some(("unreserve", game_id))) => {
            let game_id: u64 = game_id
                .parse()
                .or::<worker::Error>(Err("Failed to parse game id".into()))?;
            handle_unreserve_interaction(interaction, db, game_id).await
        }
        _ => Err("Unknown interaction".into()),
    };
}

#[event(fetch)]
async fn main(req: Request, env: Env, ctx: Context) -> Result<Response> {
    let db = env.d1("DB")?;
    let public_key = env.var("public_key")?;
    let Ok(body) = check_request(req, &public_key.to_string()).await else {
        return Response::error("", 401);
    };

    let interaction: Interaction =
        serenity::json::from_slice(&body).map_err(|err| err.to_string())?;

    let interaction_response = match interaction {
        Interaction::Ping(_) => CreateInteractionResponse::Pong,
        Interaction::Command(command) => match command.data.name.as_str() {
            "reservations" => {
                CreateInteractionResponse::Message(reservations_command(command, &db).await?)
            }
            _ => return Response::error("Unrecognized command name", 400),
        },
        Interaction::Autocomplete(_) => return Response::ok(""),
        Interaction::Component(interaction) => {
            handle_component_interaction(&interaction, &db).await?
        }
        Interaction::Modal(_) => return Response::ok(""),
        _ => return Response::ok(""),
    };

    return Response::ok(
        serenity::json::to_string(&interaction_response).map_err(|err| err.to_string())?,
    );
}
