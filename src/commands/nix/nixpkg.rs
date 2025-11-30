use crate::types::Context;
use color_eyre::eyre::{Result, eyre};
use once_cell::sync::Lazy;
use poise::{CreateReply, serenity_prelude::CreateEmbed};
use rusqlite::{Connection, params};
use std::sync::Mutex;

static DB: Lazy<Mutex<Connection>> = Lazy::new(|| {
    let db_path = std::env::var("NIXPKGS_DB").unwrap_or_else(|_| "nixpkgs.db".to_string());
    Mutex::new(Connection::open(db_path).expect("Failed to open database"))
});

#[derive(Debug)]
struct Package {
    pname: String,
    version: String,
    meta: PackageMeta,
}

#[derive(Debug)]
struct PackageMeta {
    description: String,
    homepage: Option<String>,
    license: License,
    maintainers: Vec<Maintainers>,
    position: String,
    broken: bool,
    insecure: bool,
    unfree: bool,
}

#[derive(Debug)]
struct License {
    spdx_id: String,
}

#[derive(Debug)]
struct Maintainers {
    name: String,
    github: String,
}

/// Get information about a Nix package
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn nixpkg(
    ctx: Context<'_>,
    #[description = "package name"] package: String,
) -> Result<()> {
    ctx.defer().await?;

    let (mut pkg, maintainers) = {
        let db = DB.lock().unwrap();

        let mut stmt = db.prepare(
            "SELECT pname, version, description, homepage, license_spdx_id, 
                    position, broken, insecure, unfree 
             FROM packages WHERE package_name = ?1",
        )?;

        let pkg = stmt
            .query_row(params![&package], |row| {
                Ok(Package {
                    pname: row.get(0)?,
                    version: row.get(1)?,
                    meta: PackageMeta {
                        description: row.get(2)?,
                        homepage: row.get(3)?,
                        license: License {
                            spdx_id: row
                                .get::<_, Option<String>>(4)?
                                .unwrap_or_else(|| "Unknown".to_string()),
                        },
                        position: row
                            .get::<_, Option<String>>(5)?
                            .unwrap_or_else(|| "unknown".to_string()),
                        broken: row.get::<_, i32>(6)? != 0,
                        insecure: row.get::<_, i32>(7)? != 0,
                        unfree: row.get::<_, i32>(8)? != 0,
                        maintainers: Vec::new(),
                    },
                })
            })
            .map_err(|_| eyre!("Package not found"))?;

        let mut maint_stmt =
            db.prepare("SELECT name, github FROM maintainers WHERE package_name = ?1")?;

        let maintainers: Vec<Maintainers> = maint_stmt
            .query_map(params![&package], |row| {
                Ok(Maintainers {
                    name: row
                        .get::<_, Option<String>>(0)?
                        .unwrap_or_else(|| "Unknown".to_string()),
                    github: row
                        .get::<_, Option<String>>(1)?
                        .unwrap_or_else(|| "".to_string()),
                })
            })?
            .filter_map(Result::ok)
            .collect();

        (pkg, maintainers)
    };

    pkg.meta.maintainers = maintainers;

    let file = pkg.meta.position.split(':').next().unwrap_or("unknown");

    let embed = CreateEmbed::new()
        .title(format!("{} {}", pkg.pname, pkg.version))
        .url(format!(
            "https://github.com/nixos/nixpkgs/blob/master/{file}"
        ))
        .description(pkg.meta.description)
        .field(
            "Homepage",
            pkg.meta.homepage.unwrap_or_else(|| "N/A".to_string()),
            false,
        )
        .field("license", pkg.meta.license.spdx_id, true)
        .field("insecure", pkg.meta.insecure.to_string(), true)
        .field("unfree", pkg.meta.unfree.to_string(), true)
        .field("broken", pkg.meta.broken.to_string(), true)
        .field(
            "maintainers",
            if pkg.meta.maintainers.is_empty() {
                "None".to_string()
            } else {
                pkg.meta
                    .maintainers
                    .iter()
                    .filter(|m| !m.github.is_empty())
                    .map(|m| format!("[{}](https://github.com/{})", m.name, m.github))
                    .collect::<Vec<String>>()
                    .join(", ")
            },
            false,
        )
        .color(0x00DE_A586);

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}
