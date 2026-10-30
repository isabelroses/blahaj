use crate::types::Context;
use crate::utils::DB;
use chrono::Offset;
use chrono_tz::{TZ_VARIANTS, Tz};
use color_eyre::eyre::Result;
use poise::CreateReply;
use poise::serenity_prelude::CreateEmbed;

/// Discord's limit for an embed description, with some headroom for the
/// truncation note we may append.
const EMBED_DESCRIPTION_LIMIT: usize = 4000;

/// Discord only renders the first ten embeds of a message.
const MAX_EMBEDS: usize = 10;

fn safe_reply() -> CreateReply {
    CreateReply::default().allowed_mentions(
        poise::serenity_prelude::CreateAllowedMentions::new()
            .everyone(false)
            .all_users(false)
            .all_roles(false)
            .replied_user(false)
            .empty_users()
            .empty_roles(),
    )
}

/// Resolves a user-supplied timezone name to a canonical IANA zone.
///
/// Exact IANA names are preferred, but a case-insensitive match against the
/// known zones is accepted so `europe/london` still works.
fn normalize_timezone(input: &str) -> Option<Tz> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    if let Ok(tz) = input.parse::<Tz>() {
        return Some(tz);
    }

    TZ_VARIANTS
        .iter()
        .copied()
        .find(|tz| tz.name().eq_ignore_ascii_case(input))
}

/// Formats a UTC offset in seconds as `UTC+HH:MM` / `UTC-HH:MM`.
fn format_offset(seconds: i32) -> String {
    let sign = if seconds < 0 { '-' } else { '+' };
    let abs = seconds.abs();
    format!("UTC{sign}{:02}:{:02}", abs / 3600, (abs % 3600) / 60)
}

fn upsert_timezone(guild_id: u64, user_id: u64, timezone: &str) -> Result<()> {
    let conn = DB.lock().unwrap();
    conn.execute(
        "INSERT INTO user_timezones (guild_id, user_id, timezone, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(guild_id, user_id)
         DO UPDATE SET timezone = excluded.timezone, updated_at = excluded.updated_at",
        rusqlite::params![
            guild_id.cast_signed(),
            user_id.cast_signed(),
            timezone,
            chrono::Utc::now().timestamp()
        ],
    )?;
    Ok(())
}

fn remove_timezone(guild_id: u64, user_id: u64) -> Result<bool> {
    let conn = DB.lock().unwrap();
    let removed = conn.execute(
        "DELETE FROM user_timezones WHERE guild_id = ?1 AND user_id = ?2",
        rusqlite::params![guild_id.cast_signed(), user_id.cast_signed()],
    )?;
    Ok(removed > 0)
}

fn list_timezones(guild_id: u64) -> Result<Vec<(u64, String)>> {
    let conn = DB.lock().unwrap();
    let mut stmt =
        conn.prepare("SELECT user_id, timezone FROM user_timezones WHERE guild_id = ?1")?;
    let rows = stmt
        .query_map([guild_id.cast_signed()], |row| {
            Ok((row.get::<_, i64>(0)?.cast_unsigned(), row.get(1)?))
        })?
        .collect::<rusqlite::Result<Vec<(u64, String)>>>()?;
    Ok(rows)
}

/// Splits lines into embed descriptions that stay under Discord's limit.
fn chunk_lines(lines: &[String]) -> Vec<String> {
    let mut chunks = Vec::new();
    let mut current = String::new();

    for line in lines {
        let added = line.len() + usize::from(!current.is_empty());
        if !current.is_empty() && current.len() + added > EMBED_DESCRIPTION_LIMIT {
            chunks.push(std::mem::take(&mut current));
        }

        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);
    }

    if !current.is_empty() {
        chunks.push(current);
    }

    if chunks.len() > MAX_EMBEDS {
        let dropped = chunks.len() - MAX_EMBEDS;
        chunks.truncate(MAX_EMBEDS);
        if let Some(last) = chunks.last_mut() {
            last.push_str(&format!("\n…and {dropped} more."));
        }
    }

    chunks
}

async fn autocomplete_timezone<'a>(
    _ctx: Context<'_>,
    partial: &'a str,
) -> impl Iterator<Item = String> + 'a {
    let partial = partial.to_ascii_lowercase();
    TZ_VARIANTS
        .iter()
        .map(|tz| tz.name())
        .filter(move |name| name.to_ascii_lowercase().contains(&partial))
        .take(25)
        .map(str::to_string)
}

/// Show and manage the timezones of people in this server.
#[allow(clippy::unused_async)]
#[poise::command(slash_command, guild_only, subcommands("add", "list", "remove"))]
pub async fn timezone(_: Context<'_>) -> Result<()> {
    Ok(())
}

/// Save your timezone so others can see what time it is for you.
#[poise::command(slash_command, guild_only)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "Your timezone as an IANA name, e.g. Europe/London"]
    #[autocomplete = "autocomplete_timezone"]
    timezone: String,
) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            safe_reply()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let Some(tz) = normalize_timezone(&timezone) else {
        ctx.send(
            safe_reply()
                .content(format!(
                    "`{timezone}` isn't a timezone I recognise. Use an IANA name like \
                     `Europe/London` or `America/New_York`."
                ))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    upsert_timezone(guild_id.get(), ctx.author().id.get(), tz.name())?;

    let local = chrono::Utc::now().with_timezone(&tz);
    ctx.send(
        safe_reply()
            .content(format!(
                "Saved your timezone as `{}` — it's currently {} for you.",
                tz.name(),
                local.format("%H:%M %Z")
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// List the timezones everyone in this server has set.
#[poise::command(slash_command, guild_only)]
pub async fn list(ctx: Context<'_>) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            safe_reply()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let now = chrono::Utc::now();
    let mut entries = list_timezones(guild_id.get())?
        .into_iter()
        .filter_map(|(user_id, name)| {
            let tz: Tz = name.parse().ok()?;
            let local = now.with_timezone(&tz);
            let offset = local.offset().fix().local_minus_utc();
            Some((offset, tz.name().to_string(), user_id, local))
        })
        .collect::<Vec<_>>();

    if entries.is_empty() {
        ctx.send(
            safe_reply()
                .content("No one has set a timezone here yet. Use `/timezone add` to be the first!")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    // Order by current offset so the list reads like a clock across the world.
    entries.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then_with(|| a.1.cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });

    let lines = entries
        .into_iter()
        .map(|(offset, name, user_id, local)| {
            format!(
                "<@{user_id}> — `{name}` — {} ({})",
                local.format("%H:%M %Z"),
                format_offset(offset)
            )
        })
        .collect::<Vec<_>>();

    let embeds = chunk_lines(&lines)
        .into_iter()
        .map(|description| CreateEmbed::new().description(description))
        .collect::<Vec<_>>();

    let mut reply = safe_reply().content("**Timezones in this server**");
    reply.embeds = embeds;
    ctx.send(reply).await?;

    Ok(())
}

/// Stop sharing your timezone in this server.
#[poise::command(slash_command, guild_only)]
pub async fn remove(ctx: Context<'_>) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            safe_reply()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let removed = remove_timezone(guild_id.get(), ctx.author().id.get())?;
    let message = if removed {
        "Your timezone has been removed from this server."
    } else {
        "You don't have a timezone saved in this server."
    };

    ctx.send(safe_reply().content(message).ephemeral(true))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn normalize_accepts_exact_iana_names() {
        assert_eq!(
            normalize_timezone("Europe/London").map(Tz::name),
            Some("Europe/London")
        );
        assert_eq!(normalize_timezone("UTC").map(Tz::name), Some("UTC"));
    }

    #[test]
    fn normalize_is_case_insensitive_and_trims() {
        assert_eq!(
            normalize_timezone("  europe/london  ").map(Tz::name),
            Some("Europe/London")
        );
    }

    #[test]
    fn normalize_rejects_unknown_and_empty() {
        assert!(normalize_timezone("").is_none());
        assert!(normalize_timezone("   ").is_none());
        assert!(normalize_timezone("not a timezone").is_none());
    }

    #[test]
    fn format_offset_handles_signs_and_half_hours() {
        assert_eq!(format_offset(0), "UTC+00:00");
        assert_eq!(format_offset(3600), "UTC+01:00");
        assert_eq!(format_offset(19_800), "UTC+05:30");
        assert_eq!(format_offset(-14_400), "UTC-04:00");
        assert_eq!(format_offset(-12_600), "UTC-03:30");
    }

    #[test]
    fn local_time_respects_dst() {
        let tz: Tz = "America/New_York".parse().unwrap();
        let winter = chrono::Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap();
        let summer = chrono::Utc.with_ymd_and_hms(2026, 7, 15, 12, 0, 0).unwrap();

        assert_eq!(
            winter.with_timezone(&tz).format("%H:%M").to_string(),
            "07:00"
        );
        assert_eq!(
            summer.with_timezone(&tz).format("%H:%M").to_string(),
            "08:00"
        );
    }

    #[test]
    fn chunk_lines_splits_at_the_description_limit() {
        let line = "x".repeat(1000);
        let lines = vec![line; 10];
        let chunks = chunk_lines(&lines);

        assert!(chunks.len() > 1);
        assert!(
            chunks
                .iter()
                .all(|chunk| chunk.len() <= EMBED_DESCRIPTION_LIMIT)
        );
        assert_eq!(chunks.concat().matches('x').count(), 10_000);
    }

    #[test]
    fn chunk_lines_caps_the_embed_count() {
        let line = "y".repeat(EMBED_DESCRIPTION_LIMIT);
        let lines = vec![line; MAX_EMBEDS + 3];
        let chunks = chunk_lines(&lines);

        assert_eq!(chunks.len(), MAX_EMBEDS);
        assert!(chunks.last().unwrap().contains("and 3 more"));
    }
}
