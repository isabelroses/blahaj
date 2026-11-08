pub mod bot;
pub mod fun;
pub mod misc;
pub mod moderation;
pub mod nix;
pub mod user;

use crate::types::Data;

/// Every slash command the bot registers.
pub fn all() -> Vec<poise::Command<Data, color_eyre::eyre::Report>> {
    vec![
        // user commands
        user::whois::whois(),
        user::avatar::avatar(),
        user::color_me::color_me(),
        user::timezone::timezone(),
        // bot commands
        bot::ping::ping(),
        bot::bot::botinfo(),
        // misc commands
        misc::avatarsync::avatarsync(),
        misc::crates::crates(),
        misc::starboard::starboard_enable(),
        misc::starboard::starboard_disable(),
        misc::starboard::starboard_config(),
        misc::typst::typst(),
        // moderation commands
        moderation::ban::ban(),
        moderation::kick::kick(),
        moderation::purge::purge(),
        moderation::timeout::timeout(),
        // commands for nix
        nix::nixpkgs::nixpkgs(),
        nix::nix::nix(),
        nix::track::nixpkgs_track(),
        // fun commands
        fun::chance::roll(),
        fun::kittysay::kittysay(),
        fun::bottom::topify(),
        fun::bottom::bottomify(),
        fun::pet::pet(),
        fun::height::height(),
        fun::they::they(),
    ]
}
