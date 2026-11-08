use std::sync::LazyLock;

use color_eyre::eyre::Result;
use poise::serenity_prelude::{Context, FullEvent};
use regex::Regex;
use serenity::all::EditMessage;
use url::Url;

use crate::types::Data;

static LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(https?:\/\/(?:(www|vm)\.)?(x\.com|twitter\.com|reddit\.com|instagram\.com|tiktok\.com)\/[^\s]+)").unwrap()
});

const REPLACEMENT_HOSTS: &[&str] = &[
    "vxtwitter.com",
    "vxreddit.com",
    "oginstagram.com",
    "tnktok.com",
    "vm.tnktok.com",
];

fn is_tracking_parameter(name: &str) -> bool {
    name.get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("utm_"))
        || matches!(
            name.to_ascii_lowercase().as_str(),
            "fbclid"
                | "gclid"
                | "dclid"
                | "gbraid"
                | "wbraid"
                | "msclkid"
                | "mc_cid"
                | "mc_eid"
                | "_hsenc"
                | "_hsmi"
        )
}

fn strip_query_parameters(url: &str, should_strip: impl Fn(&str) -> bool) -> String {
    let mut parsed_url = match Url::parse(url) {
        Ok(url) => url,
        Err(err) => {
            eprintln!("failed to parse replacement URL {url}: {err}");
            return url.to_string();
        }
    };

    let parameters: Vec<_> = parsed_url
        .query_pairs()
        .filter(|(name, _)| !should_strip(name))
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();

    if parameters.is_empty() {
        parsed_url.set_query(None);
    } else {
        let mut query_pairs = parsed_url.query_pairs_mut();
        query_pairs.clear();
        for (name, value) in parameters {
            query_pairs.append_pair(&name, &value);
        }
    }

    parsed_url.to_string()
}

fn is_twitter_url(url: &str) -> bool {
    url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .and_then(|url| url.split('/').next())
        .is_some_and(|host| {
            matches!(
                host.strip_prefix("www.").unwrap_or(host),
                "x.com" | "twitter.com"
            )
        })
}

pub fn is_link_replacement(content: &str) -> bool {
    let mut tokens = content.split_whitespace().peekable();
    if tokens.peek().is_none() {
        return false;
    }

    tokens.all(|token| {
        token
            .strip_prefix("https://")
            .map(|rest| rest.split('/').next().unwrap_or_default())
            .is_some_and(|host| REPLACEMENT_HOSTS.contains(&host))
    })
}

pub async fn handle(ctx: &Context, event: &FullEvent, _data: &Data) -> Result<()> {
    if let FullEvent::Message { new_message } = event {
        if !new_message.content.contains("://") {
            return Ok(());
        }
        let mut links: Vec<String> = Vec::new();

        let content = &new_message.content;
        for capture in LINK_RE.find_iter(content) {
            let url = capture.as_str();

            // Skip links prefixed with `!` so users can opt out of embed
            // replacement by writing e.g. "!https://twitter.com/foo".
            if capture.start() > 0 && content.as_bytes()[capture.start() - 1] == b'!' {
                continue;
            }

            let modified_url = url
                .replace("https://x.com", "https://vxtwitter.com")
                .replace("https://twitter.com", "https://vxtwitter.com")
                .replace("https://www.reddit.com", "https://vxreddit.com")
                .replace("https://reddit.com", "https://vxreddit.com")
                .replace("https://www.instagram.com", "https://oginstagram.com")
                .replace("https://instagram.com", "https://oginstagram.com")
                .replace("https://www.tiktok.com", "https://tnktok.com")
                .replace("https://vm.tiktok.com", "https://vm.tnktok.com")
                .replace("https://tiktok.com", "https://tnktok.com");

            links.push(if is_twitter_url(url) {
                strip_query_parameters(&modified_url, |_| true)
            } else {
                strip_query_parameters(&modified_url, is_tracking_parameter)
            });
        }

        let message_id = new_message.id;
        let channel_id = new_message.channel_id;

        if !links.is_empty() {
            let _ = channel_id
                .edit_message(
                    ctx.http.clone(),
                    message_id,
                    EditMessage::new().suppress_embeds(true),
                )
                .await;
            let _ = new_message.reply(ctx.http.clone(), links.join("\n")).await;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_our_own_replacement_messages() {
        assert!(is_link_replacement("https://vxtwitter.com/foo/status/1"));
        assert!(is_link_replacement(
            "https://vxreddit.com/r/a\nhttps://vm.tnktok.com/b"
        ));
    }

    #[test]
    fn rejects_anything_that_is_not_purely_replaced_links() {
        assert!(!is_link_replacement(""));
        assert!(!is_link_replacement("   \n "));
        assert!(!is_link_replacement(
            "look at this https://vxtwitter.com/foo"
        ));
        assert!(!is_link_replacement("https://x.com/foo"));
        assert!(!is_link_replacement("https://vxtwitter.com.evil.com/foo"));
        assert!(!is_link_replacement("sure, here's the answer"));
    }

    #[test]
    fn strips_tracking_parameters_from_replaced_links() {
        assert_eq!(
            strip_query_parameters(
                "https://vxtwitter.com/foo/status/1?utm_source=discord&lang=en&fbclid=abc#reply",
                is_tracking_parameter
            ),
            "https://vxtwitter.com/foo/status/1?lang=en#reply"
        );
    }

    #[test]
    fn removes_the_query_separator_when_only_tracking_parameters_remain() {
        assert_eq!(
            strip_query_parameters(
                "https://vxreddit.com/r/a?gclid=abc&utm_medium=social",
                is_tracking_parameter
            ),
            "https://vxreddit.com/r/a"
        );
    }

    #[test]
    fn strips_all_query_parameters_from_twitter_links() {
        assert_eq!(
            strip_query_parameters(
                "https://vxtwitter.com/foo/status/1?lang=en&view=1#reply",
                |_| true
            ),
            "https://vxtwitter.com/foo/status/1#reply"
        );
        assert!(is_twitter_url("https://www.x.com/foo/status/1?lang=en"));
        assert!(is_twitter_url("http://twitter.com/foo/status/1"));
    }
}
