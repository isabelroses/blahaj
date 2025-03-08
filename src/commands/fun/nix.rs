use crate::types::Context;
use color_eyre::eyre::Result;
use rand::Rng;

#[allow(dead_code)]
const MEMES: &[&str] = &[
    "the-declarative-trinity.webp",
    "my-nixos-setup.png",
    "before-and-after-nix.png",
    "hard-to-swallow-pills.png",
    "i-hate-docker.webp",
    "just-try-the-goddam-nix.webp",
    "nix-learning-curve.png",
    "nix-vs-gentoo.png",
    "nixos-deploy.png",
    "no-going-back.png",
    "random-repos.png",
    "whats-the-difference.webp",
    "nixos-dominos.png",
    "nix-path-supports-urls.jpg",
    "virtualbox-starts-compiling.jpg",
    "stop-using-nixos.webp",
    "config-not-entierly-declarative.png",
    "debian-and-arch-bad.png",
    "do-not-get-mad.png",
    "eelco-nixpill.png",
    "eelco-prism.apng",
    "fleyks.png",
    "mobile-nixos.png",
    "who-would-win.png",
    "nixos-shilling.png",
    "techy-kid.png",
    "nix-vs-fhs.png",
    "quick-install-nixos.webp",
    "homer-nix-bush.gif",
    "superiority-complex.png",
    "nix-programming-socks.png",
    "pinnacle-of-system-configuration.png",
    "thank-you-for-changing-my-life.png",
    "virgin-arch-vs-chad-nixos.png",
    "heaviest-objects-in-the-universe.png",
    "nagatoro-nix-pervert.png",
    "nix-20min-adventure.png",
    "nixenv-vs-nixshell.png",
    "org-vs-com.png",
    "hermetic-tooling.jpg",
    "nixos-at-home.jpg",
    "aarch64-joke.jpg",
    "dark-secret-nixpkgs.png",
    "electron.jpg",
    "pr-open.jpg",
    "stay-on-freenode.jpg",
    "they-dont-know-im-reproducible.png",
    "nix-god.jpg",
    "flake-magic.png",
    "averagenixfan.png",
    "legend-of-nixos.png",
];

/// nix fixes this
#[poise::command(slash_command)]
pub async fn nix(ctx: Context<'_>) -> Result<()> {
    let select = rand::rng().random_range(0..=MEMES.len());
    let img = MEMES[select];

    ctx.say(format!("https://raw.githubusercontent.com/gytis-ivaskevicius/high-quality-nix-content/master/memes/{img}")).await?;
    Ok(())
}
