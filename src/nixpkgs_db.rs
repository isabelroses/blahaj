use color_eyre::eyre::Result;
use sha2::{Digest, Sha256};
use std::path::Path;

#[derive(Debug)]
pub struct NixpkgsRelease {
    pub url: String,
    pub hash: String,
}

#[derive(Debug, Clone)]
struct Package {
    name: String,
    pname: Option<String>,
    version: Option<String>,
    display_name: Option<String>,
    system: Option<String>,
    output_name: Option<String>,
    available: i32,
    broken: i32,
    description: Option<String>,
    homepage: Option<String>,
    insecure: i32,
    unfree: i32,
    unsupported: i32,
    position: Option<String>,
    long_description: Option<String>,
    main_program: Option<String>,
    license_spdx_id: Option<String>,
    license_full_name: Option<String>,
    license_free: i32,
    license_url: Option<String>,
}

#[derive(Debug, Clone)]
struct Maintainer {
    package_name: String,
    name: Option<String>,
    email: Option<String>,
    github: Option<String>,
    github_id: Option<i64>,
    matrix: Option<String>,
}

pub async fn get_latest_nixpkgs_release() -> Result<NixpkgsRelease> {
    let base_url = crate::config::get().nixpkgs_channel.clone();

    let response = reqwest::get(&base_url).await?;
    let html = response.text().await?;

    let url_regex =
        regex::Regex::new(r"<a href='([^']+/packages\.json\.br)'>packages\.json\.br</a>")?;
    let hash_regex = regex::Regex::new(
        r"packages\.json\.br</a></td><td align='right'>\d+</td><td><tt>([a-f0-9]{64})</tt>",
    )?;

    let url = url_regex
        .captures(&html)
        .and_then(|cap| cap.get(1))
        .map(|m| {
            let path = m.as_str();
            if path.starts_with("http") {
                path.to_string()
            } else if path.starts_with('/') {
                format!("https://releases.nixos.org{path}")
            } else {
                format!("https://releases.nixos.org/{path}")
            }
        })
        .ok_or_else(|| color_eyre::eyre::eyre!("Could not find packages.json.br URL"))?;

    let hash = hash_regex
        .captures(&html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| color_eyre::eyre::eyre!("Could not find packages.json.br hash"))?;

    Ok(NixpkgsRelease { url, hash })
}

fn get_stored_hash() -> Option<String> {
    let hash_path = crate::utils::get_data_dir().join("nixpkgs.hash");
    std::fs::read_to_string(hash_path).ok()
}

fn store_hash(hash: &str) -> Result<()> {
    let hash_path = crate::utils::get_data_dir().join("nixpkgs.hash");
    std::fs::write(hash_path, hash)?;
    Ok(())
}

#[allow(clippy::too_many_lines, clippy::items_after_statements)]
pub async fn ensure_nixpkgs_database() -> Result<()> {
    let db_path = crate::utils::get_data_dir().join("packages.db");

    println!("Checking for nixpkgs updates...");
    let release = get_latest_nixpkgs_release().await?;
    let stored_hash = get_stored_hash();

    if Path::new(&db_path).exists() && stored_hash.as_deref() == Some(&release.hash) {
        println!("nixpkgs database is up to date");
        return Ok(());
    }

    if Path::new(&db_path).exists() {
        println!("New nixpkgs release detected, updating database...");
        std::fs::remove_file(&db_path)?;
    } else {
        println!("nixpkgs database not found, building...");
    }

    println!("Downloading from {}...", release.url);
    let response = reqwest::get(&release.url).await?;
    let compressed = response.bytes().await?;

    let mut hasher = Sha256::new();
    hasher.update(&compressed);
    let computed_hash = format!("{:x}", hasher.finalize());

    if computed_hash != release.hash {
        return Err(color_eyre::eyre::eyre!(
            "Hash mismatch! Expected {}, got {}",
            release.hash,
            computed_hash
        ));
    }

    println!("Hash verified, decompressing...");
    let mut decompressed = Vec::new();
    let mut decoder = brotli::Decompressor::new(compressed.as_ref(), 4096);
    std::io::copy(&mut decoder, &mut decompressed)?;

    println!("Parsing JSON...");
    let json_data: serde_json::Value = serde_json::from_slice(&decompressed)?;

    let packages = json_data["packages"]
        .as_object()
        .ok_or_else(|| color_eyre::eyre::eyre!("Invalid packages.json format"))?;

    println!("Creating database with {} packages...", packages.len());

    const BATCH_SIZE: usize = 5000; // Increased from 1000 for better performance

    let mut conn = rusqlite::Connection::open(&db_path)?;

    conn.execute(
        "CREATE TABLE packages (
            package_name TEXT PRIMARY KEY,
            pname TEXT,
            version TEXT,
            name TEXT,
            system TEXT,
            output_name TEXT,
            available INTEGER,
            broken INTEGER,
            description TEXT,
            homepage TEXT,
            insecure INTEGER,
            unfree INTEGER,
            unsupported INTEGER,
            position TEXT,
            long_description TEXT,
            main_program TEXT,
            license_spdx_id TEXT,
            license_full_name TEXT,
            license_free INTEGER,
            license_url TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE maintainers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            package_name TEXT,
            name TEXT,
            email TEXT,
            github TEXT,
            github_id INTEGER,
            matrix TEXT,
            FOREIGN KEY (package_name) REFERENCES packages(package_name)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX idx_package_name ON packages(package_name)",
        [],
    )?;
    conn.execute("CREATE INDEX idx_pname ON packages(pname)", [])?;
    conn.execute(
        "CREATE INDEX idx_maintainers_package ON maintainers(package_name)",
        [],
    )?;

    // Enable WAL mode for better concurrent access
    conn.pragma_update(None, "journal_mode", "WAL")?;

    // Increase cache size for better performance
    conn.pragma_update(None, "cache_size", "-64000")?;

    let total = packages.len();
    let mut count = 0;

    let mut package_batch = Vec::with_capacity(BATCH_SIZE);
    let mut maintainer_batch = Vec::with_capacity(BATCH_SIZE * 4); // Estimate 4 maintainers per package

    for (pkg_name, pkg_data) in packages {
        let meta = &pkg_data["meta"];
        let license_data = &meta["license"];

        let license_spdx = extract_license(license_data);
        let homepage = extract_homepage(meta);

        package_batch.push(Package {
            name: pkg_name.clone(),
            pname: extract_string(pkg_data, "pname"),
            version: extract_string(pkg_data, "version"),
            display_name: extract_string(pkg_data, "name"),
            system: extract_string(pkg_data, "system"),
            output_name: extract_string(pkg_data, "outputName"),
            available: i32::from(
                meta.get("available")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            ),
            broken: i32::from(
                meta.get("broken")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            ),
            description: extract_string(meta, "description"),
            homepage,
            insecure: i32::from(
                meta.get("insecure")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            ),
            unfree: i32::from(
                meta.get("unfree")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            ),
            unsupported: i32::from(
                meta.get("unsupported")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            ),
            position: extract_string(meta, "position"),
            long_description: extract_string(meta, "longDescription"),
            main_program: extract_string(meta, "mainProgram"),
            license_spdx_id: license_spdx,
            license_full_name: None,
            license_free: 0,
            license_url: None,
        });

        if let Some(maintainers) = meta.get("maintainers").and_then(|v| v.as_array()) {
            for m in maintainers {
                if let Some(obj) = m.as_object() {
                    maintainer_batch.push(Maintainer {
                        package_name: pkg_name.clone(),
                        name: obj
                            .get("name")
                            .and_then(|v| v.as_str())
                            .map(std::string::ToString::to_string),
                        email: obj
                            .get("email")
                            .and_then(|v| v.as_str())
                            .map(std::string::ToString::to_string),
                        github: obj
                            .get("github")
                            .and_then(|v| v.as_str())
                            .map(std::string::ToString::to_string),
                        github_id: obj.get("githubId").and_then(serde_json::Value::as_i64),
                        matrix: obj
                            .get("matrix")
                            .and_then(|v| v.as_str())
                            .map(std::string::ToString::to_string),
                    });
                }
            }
        }

        count += 1;

        if package_batch.len() >= BATCH_SIZE {
            insert_batch(&mut conn, &package_batch, &maintainer_batch)?;
            print_progress(count, total);
            package_batch.clear();
            maintainer_batch.clear();
        }
    }

    if !package_batch.is_empty() {
        insert_batch(&mut conn, &package_batch, &maintainer_batch)?;
    }

    println!("Vacuuming...");
    conn.execute("VACUUM", [])?;

    store_hash(&release.hash)?;

    println!("Database created successfully: {}", db_path.display());
    Ok(())
}

/// Extract a string value from a JSON object
fn extract_string(obj: &serde_json::Value, key: &str) -> Option<String> {
    obj.get(key)
        .and_then(serde_json::Value::as_str)
        .map(std::string::ToString::to_string)
}

/// Extract license information from license data
fn extract_license(license_data: &serde_json::Value) -> Option<String> {
    match license_data {
        serde_json::Value::Object(obj) => obj
            .get("spdxId")
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string),
        serde_json::Value::Array(arr) => {
            let ids: Vec<&str> = arr
                .iter()
                .filter_map(|v| v.get("spdxId"))
                .filter_map(|v| v.as_str())
                .collect();
            if ids.is_empty() {
                None
            } else {
                Some(ids.join(", "))
            }
        }
        serde_json::Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// Extract homepage from metadata
fn extract_homepage(meta: &serde_json::Value) -> Option<String> {
    meta.get("homepage").and_then(|h| match h {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Array(arr) => arr
            .first()
            .and_then(|v| v.as_str())
            .map(std::string::ToString::to_string),
        _ => None,
    })
}

/// Insert a batch of packages and maintainers into the database
fn insert_batch(
    conn: &mut rusqlite::Connection,
    package_batch: &[Package],
    maintainer_batch: &[Maintainer],
) -> Result<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare_cached("INSERT INTO packages VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")?;
        for p in package_batch {
            stmt.execute(rusqlite::params![
                p.name,
                p.pname,
                p.version,
                p.display_name,
                p.system,
                p.output_name,
                p.available,
                p.broken,
                p.description,
                p.homepage,
                p.insecure,
                p.unfree,
                p.unsupported,
                p.position,
                p.long_description,
                p.main_program,
                p.license_spdx_id,
                p.license_full_name,
                p.license_free,
                p.license_url,
            ])?;
        }
    }
    {
        let mut stmt = tx.prepare_cached("INSERT INTO maintainers (package_name, name, email, github, github_id, matrix) VALUES (?, ?, ?, ?, ?, ?)")?;
        for m in maintainer_batch {
            stmt.execute(rusqlite::params![
                m.package_name,
                m.name,
                m.email,
                m.github,
                m.github_id,
                m.matrix,
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

/// Print progress information
fn print_progress(count: usize, total: usize) {
    #[allow(clippy::cast_precision_loss)]
    let progress = (count as f64 / total as f64) * 100.0;
    println!("Progress: {count}/{total} ({progress:.1}%)");
}
