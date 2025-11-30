use crate::types::Context;
use color_eyre::eyre::Result;
use rand::Rng;

#[allow(dead_code)]
const MEMES: &[&str] = &[
    // memes from github:gytis-ivaskevicius/high-quality-nix-content
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/aarch64-joke.jpg",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/averagenixfan.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/before-and-after-nix.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/config-not-entierly-declarative.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/dark-secret-nixpkgs.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/debian-and-arch-bad.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/do-not-get-mad.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/eelco-nixpill.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/eelco-prism.mp4",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/electron.jpg",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/flake-magic.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/fleyks.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/hard-to-swallow-pills.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/heaviest-objects-in-the-universe.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/hermetic-tooling.jpg",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/homer-nix-bush.gif",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/i-hate-docker.webp",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/just-try-the-goddam-nix.webp",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/legend-of-nixos.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/mobile-nixos.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/my-nixos-setup.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nagatoro-nix-pervert.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nix-20min-adventure.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nix-expression-language-armor.jpeg",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nix-god.jpg",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nix-learning-curve.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nix-path-supports-urls.jpg",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nix-programming-socks.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nix-vs-fhs.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nix-vs-gentoo.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nixenv-vs-nixshell.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nixos-at-home.jpg",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nixos-deploy.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nixos-dominos.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nixos-fixes-this.jpg",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/nixos-shilling.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/no-going-back.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/org-vs-com.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/pinnacle-of-system-configuration.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/pr-open.jpg",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/quick-install-nixos.webp",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/random-repos.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/stay-on-freenode.jpg",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/stop-using-nixos.webp",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/superiority-complex.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/techy-kid.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/thank-you-for-changing-my-life.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/the-declarative-trinity.webp",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/they-dont-know-im-reproducible.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/virgin-arch-vs-chad-nixos.png",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/virtualbox-starts-compiling.jpg",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/whats-the-difference.webp",
    "https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/who-would-win.png",
    // memes from github:isabelroses/memes
    "https://media.githubusercontent.com/media/isabelroses/memes/refs/heads/main/nix-fixes-this/nix-fixes-this.png",
    "https://media.githubusercontent.com/media/isabelroses/memes/refs/heads/main/nix-fixes-this/nix-fixes-crowdstrike.png",
    "https://media.githubusercontent.com/media/isabelroses/memes/refs/heads/main/nixgf/latest.png",
    "https://media.githubusercontent.com/media/isabelroses/memes/refs/heads/main/nix-users-today.png",
    "https://media.githubusercontent.com/media/isabelroses/memes/refs/heads/main/nix-is-a-slipper-slope.png",
];

/// nix fixes this
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn nix(ctx: Context<'_>) -> Result<()> {
    let select = rand::rng().random_range(0..=MEMES.len());
    ctx.say(MEMES[select]).await?;
    Ok(())
}
