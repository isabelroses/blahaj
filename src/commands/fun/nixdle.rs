use crate::types::Context;
use color_eyre::eyre::Result;
use nixdle::api;
use poise::CreateReply;
use sqlx::FromRow;

const API_URL: &str = "https://adamperkowski.dev/api/nixdle";

#[derive(FromRow)]
#[allow(dead_code)]
struct GameData {
    id: i32,
    date: String,
    rules: String,
}

#[derive(FromRow)]
#[allow(dead_code)]
struct UserData {
    id: i64,
    game_id: i32,
    success: bool,
    attempts: u64,
    attempted: String,
}

/// Play a game of Nixdle
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn nixdle(
    ctx: Context<'_>,
    #[description = "your guess"] guess: Option<String>,
) -> Result<()> {
    let nixus_christ = "<:nixus_christ:1392820849907732570>";

    let client = &ctx.data().client;
    let db_pool = &ctx.data().db_pool;

    let game_data = match sqlx::query_as::<_, GameData>(
        "SELECT * FROM game_data
        WHERE date = CURRENT_DATE
        ORDER BY id DESC",
    )
    .fetch_one(db_pool)
    .await
    {
        Ok(data) => data,
        Err(_) => new_game(client, db_pool).await?,
    };

    let user_id = ctx.author().id.get() as i64;
    let user_data = match sqlx::query_as::<_, UserData>(
        "SELECT * FROM user_data
        WHERE id = $1 AND game_id = $2",
    )
    .bind(user_id)
    .bind(game_data.id)
    .fetch_one(db_pool)
    .await
    {
        Ok(data) => data,
        Err(_) => new_user(user_id, game_data.id, db_pool).await?,
    };

    if user_data.success {
        ctx.send(
            CreateReply::default()
                .content(format!(
                    "You've already solved today's [Nixdle](<{}>)! {}",
                    API_URL, nixus_christ
                ))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    if let Some(guess) = guess {
        let data = api::AttemptData {
            input: guess,
            attempts: 0,
        };

        let msg = client
            .post(format!("{}/attempt", API_URL))
            .json(&data)
            .send()
            .await?
            .json::<Option<api::AttemptMessage>>()
            .await?;

        if let Some(msg) = msg {
            if msg.success {
                sqlx::query(
                    "UPDATE user_data
                    SET success = $2, attempts = $3, attempted = $4
                    WHERE id = $1",
                )
                .bind(user_id)
                .bind(true)
                .bind((user_data.attempts + 1) as i64)
                .bind({
                    let mut attempted: Vec<String> = serde_json::from_str(&user_data.attempted)?;
                    attempted.push(data.input);
                    serde_json::to_string(&attempted)?
                })
                .execute(db_pool)
                .await?;

                ctx.say(format!(
                    "Congrats! You solved today's [Nixdle](<{}>)! 🎉\nHere's your reward: 🍪",
                    API_URL
                ))
                .await?;
                ctx.send(
                    CreateReply::default()
                        .content(format!(
                            "{0} Function: {1}\n{0} Description: {2}\n{0} Date: {3}",
                            nixus_christ,
                            msg.func.unwrap_or_default(),
                            msg.description.unwrap_or_default(),
                            "a"
                        ))
                        .ephemeral(true),
                )
                .await?;

                return Ok(());
            }

            sqlx::query(
                "UPDATE user_data
                SET attempts = $2, attempted = $3
                WHERE id = $1",
            )
            .bind(user_id)
            .bind((user_data.attempts + 1) as i64)
            .bind({
                let mut attempted: Vec<&str> = serde_json::from_str(&user_data.attempted)?;
                attempted.push(&data.input);
                serde_json::to_string(&attempted)?
            })
            .execute(db_pool)
            .await?;

            let snod = "<:snod:1219050293782646865>";
            let sno = "<:sno:1219050254629081178>";

            ctx.say("Good guess! That's not it, though <:wires:1392814388913897543>")
                .await?;
            ctx.send(
                CreateReply::default()
                    .content(format!(
                        "> {}\nArguments: {}\nInput type: {}\nOutput type: {}",
                        data.input,
                        msg.args,
                        if msg.input { snod } else { sno },
                        if msg.output { snod } else { sno }
                    ))
                    .ephemeral(true),
            )
            .await?;

            return Ok(());
        }

        ctx.send(
            CreateReply::default()
                .content("The server doesn't know this one :c")
                .ephemeral(true),
        )
        .await?;

        return Ok(());
    }

    ctx.say(format!(
            "Welcome to [Nixdle](<{}>)! {}\nTry to guess today's Nix function.\n> {}\nGood luck!\n-# use `/nixdle <guess>` :p",
            API_URL,
            nixus_christ,
            &game_data.rules.replace("\n", "\n > "),
        ))
        .await?;

    Ok(())
}

async fn new_game(
    client: &reqwest::Client,
    db_pool: &sqlx::Pool<sqlx::Sqlite>,
) -> Result<GameData> {
    let msg = client
        .get(format!("{}/start", API_URL))
        .send()
        .await?
        .json::<api::StartMessage>()
        .await?;

    let data = sqlx::query_as::<_, GameData>(
        "INSERT INTO game_data (date, rules)
        VALUES ($1, $2)
        RETURNING *",
    )
    .bind(msg.date)
    .bind(msg.rules)
    .fetch_one(db_pool)
    .await?;

    Ok(data)
}

async fn new_user(
    user_id: i64,
    game_id: i32,
    db_pool: &sqlx::Pool<sqlx::Sqlite>,
) -> Result<UserData> {
    let data = sqlx::query_as::<_, UserData>(
        "INSERT INTO user_data (id, game_id, success, attempts, attempted)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *",
    )
    .bind(user_id)
    .bind(game_id)
    .bind(false)
    .bind(0)
    .bind(serde_json::json!([]).to_string())
    .fetch_one(db_pool)
    .await?;

    Ok(data)
}
