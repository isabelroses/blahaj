use crate::types::Context;
use crate::utils::DB;
use chrono::{Datelike, Offset, Timelike};
use chrono_tz::{TZ_VARIANTS, Tz};
use color_eyre::eyre::Result;
use poise::CreateReply;
use poise::serenity_prelude::{CreateAllowedMentions, CreateEmbed, CreateEmbedFooter};

const EMBED_DESCRIPTION_LIMIT: usize = 3800;

fn no_mentions() -> CreateAllowedMentions {
    CreateAllowedMentions::new()
        .everyone(false)
        .all_users(false)
        .all_roles(false)
        .replied_user(false)
        .empty_users()
        .empty_roles()
}

fn safe_reply() -> CreateReply {
    CreateReply::default().allowed_mentions(no_mentions())
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

/// Joins lines into one embed description, truncating if it would exceed
/// Discord's limit.
fn build_description(lines: &[String]) -> String {
    let mut description = String::new();
    let mut shown = 0;

    for line in lines {
        let added = line.len() + usize::from(!description.is_empty());
        if !description.is_empty() && description.len() + added > EMBED_DESCRIPTION_LIMIT {
            break;
        }

        if !description.is_empty() {
            description.push('\n');
        }
        description.push_str(line);
        shown += 1;
    }

    if shown < lines.len() {
        description.push_str(&format!("\n…and {} more.", lines.len() - shown));
    }

    description
}

struct Entry {
    user_id: u64,
    tz_name: String,
    offset: i32,
    local: chrono::DateTime<Tz>,
}

/// Reads everyone's saved timezone and works out their local time right now.
fn build_entries(guild_id: u64) -> Result<Vec<Entry>> {
    let now = chrono::Utc::now();
    let mut entries = list_timezones(guild_id)?
        .into_iter()
        .filter_map(|(user_id, name)| {
            let tz: Tz = name.parse().ok()?;
            let local = now.with_timezone(&tz);
            let offset = local.offset().fix().local_minus_utc();

            Some(Entry {
                user_id,
                tz_name: tz.name().to_string(),
                offset,
                local,
            })
        })
        .collect::<Vec<_>>();

    // Order by current offset.
    entries.sort_by(|a, b| {
        a.offset
            .cmp(&b.offset)
            .then_with(|| a.tz_name.cmp(&b.tz_name))
            .then_with(|| a.user_id.cmp(&b.user_id))
    });

    Ok(entries)
}

/// The last component of an IANA name, e.g. `Australia/Melbourne` -> `Melbourne`.
/// Underscores become spaces so `Buenos_Aires` reads as `Buenos Aires`.
fn short_zone(tz_name: &str) -> String {
    tz_name
        .rsplit('/')
        .next()
        .unwrap_or(tz_name)
        .replace('_', " ")
}

/// These are just roughly dawn / daytime / dusk / night time ranges.
fn time_emoji(hour: u32) -> &'static str {
    match hour {
        6..=8 => "🌅",
        9..=17 => "☀️",
        18..=21 => "🌆",
        _ => "🌙",
    }
}

/// Formats a date as `Tue 3 Nov` for a day header.
fn day_header(date: chrono::NaiveDate) -> String {
    format!("{} {} {}", date.format("%a"), date.day(), date.format("%b"))
}

fn render_view(entries: &[Entry], detailed: bool) -> CreateEmbed {
    // Since entries are sorted by offset, local dates only ever increase.
    // Only bother with headers when the list actually spans more than one day.
    let multi_day = entries
        .first()
        .zip(entries.last())
        .is_some_and(|(first, last)| first.local.date_naive() != last.local.date_naive());

    let mut blocks = Vec::new();
    let mut current_day = None;

    for entry in entries {
        let emoji = time_emoji(entry.local.hour());
        let line = if detailed {
            format!(
                "{} {} - <@{}> - `{}` - {} ({})",
                emoji,
                entry.local.format("%H:%M"),
                entry.user_id,
                entry.tz_name,
                entry.local.format("%Z"),
                format_offset(entry.offset)
            )
        } else {
            format!(
                "{} {} - <@{}> ({})",
                emoji,
                entry.local.format("%H:%M"),
                entry.user_id,
                short_zone(&entry.tz_name)
            )
        };

        let day = entry.local.date_naive();
        if multi_day && current_day != Some(day) {
            blocks.push(format!("**{}**\n{line}", day_header(day)));
            current_day = Some(day);
        } else {
            blocks.push(line);
        }
    }

    let footer = format!(
        "{} {} · sorted by time",
        entries.len(),
        if entries.len() == 1 {
            "person"
        } else {
            "people"
        },
    );

    CreateEmbed::new()
        .title("🕒 Timezones")
        .description(build_description(&blocks))
        .color(0x5D_81_96)
        .footer(CreateEmbedFooter::new(footer))
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
                "Saved your timezone as `{}` - it's currently {} for you.",
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
pub async fn list(
    ctx: Context<'_>,
    #[description = "Show timezone codes and UTC offsets (default: false)"] show_details: Option<
        bool,
    >,
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

    let entries = build_entries(guild_id.get())?;

    if entries.is_empty() {
        ctx.send(
            safe_reply()
                .content("No one has set a timezone here yet. Use `/timezone add` to be the first!")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let reply = safe_reply().embed(render_view(&entries, show_details.unwrap_or(false)));

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

    #[test]
    fn normalize_timezone_resolves_iana_names() {
        assert_eq!(
            normalize_timezone("Europe/London").map(Tz::name),
            Some("Europe/London")
        );
        assert_eq!(normalize_timezone("UTC").map(Tz::name), Some("UTC"));
        assert_eq!(
            normalize_timezone("  europe/london  ").map(Tz::name),
            Some("Europe/London")
        );
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
    fn short_zone_keeps_only_the_last_component() {
        assert_eq!(short_zone("Australia/Melbourne"), "Melbourne");
        assert_eq!(short_zone("America/Argentina/Buenos_Aires"), "Buenos Aires");
        assert_eq!(short_zone("UTC"), "UTC");
    }
}
