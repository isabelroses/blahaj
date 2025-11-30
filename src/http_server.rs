use crate::types::Data;
use poise::serenity_prelude::{Context, GuildId};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Serialize)]
struct MemberInfo {
    id: String,
    username: String,
    avatar_url: Option<String>,
}

#[derive(Serialize)]
struct MembersResponse {
    guild_id: String,
    members: Vec<MemberInfo>,
}

pub async fn start_http_server(
    ctx: Arc<Context>,
    data: Arc<Data>,
    port: u16,
) -> color_eyre::eyre::Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&addr).await?;

    println!("HTTP server listening on http://{addr}");

    loop {
        let (stream, _) = listener.accept().await?;
        let ctx = Arc::clone(&ctx);
        let data = Arc::clone(&data);

        tokio::spawn(async move {
            if let Err(e) = handle_connection(stream, ctx, data).await {
                eprintln!("Error handling connection: {e}");
            }
        });
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    ctx: Arc<Context>,
    _data: Arc<Data>,
) -> color_eyre::eyre::Result<()> {
    let mut buffer = [0; 1024];
    stream.read(&mut buffer).await?;

    let request = String::from_utf8_lossy(&buffer[..]);
    let request_line = request.lines().next().unwrap_or("");

    if request_line.starts_with("GET /members") {
        let guild_id = extract_guild_id_from_request(request_line)
            .unwrap_or(GuildId::new(1095080242219073606));

        match get_guild_members(&ctx, guild_id).await {
            Ok(response_body) => {
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    response_body.len(),
                    response_body
                );
                stream.write_all(response.as_bytes()).await?;
            }
            Err(e) => {
                let error_body = format!(r#"{{"error": "Failed to fetch members: {e}"}}"#);
                let response = format!(
                    "HTTP/1.1 500 Internal Server Error\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                    error_body.len(),
                    error_body
                );
                stream.write_all(response.as_bytes()).await?;
            }
        }
    } else {
        let not_found = r#"{"error": "Not found. Use /members?guild_id=YOUR_GUILD_ID"}"#;
        let response = format!(
            "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            not_found.len(),
            not_found
        );
        stream.write_all(response.as_bytes()).await?;
    }

    Ok(())
}

fn extract_guild_id_from_request(request_line: &str) -> Option<GuildId> {
    if let Some(query_start) = request_line.find('?') {
        let query = &request_line[query_start + 1..];
        let params: HashMap<&str, &str> = query
            .split('&')
            .filter_map(|param| {
                let mut parts = param.split('=');
                Some((parts.next()?, parts.next()?))
            })
            .collect();

        if let Some(guild_id_str) = params.get("guild_id")
            && let Ok(guild_id) = guild_id_str.parse::<u64>()
        {
            return Some(GuildId::new(guild_id));
        }
    }
    None
}

async fn get_guild_members(ctx: &Context, guild_id: GuildId) -> color_eyre::eyre::Result<String> {
    let members = guild_id.members(&ctx.http, None, None).await?;

    let member_infos: Vec<MemberInfo> = members
        .iter()
        .map(|member| MemberInfo {
            id: member.user.id.to_string(),
            username: member.user.name.clone(),
            avatar_url: member.user.avatar_url(),
        })
        .collect();

    let response = MembersResponse {
        guild_id: guild_id.to_string(),
        members: member_infos,
    };

    Ok(serde_json::to_string_pretty(&response)?)
}
