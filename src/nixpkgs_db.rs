use color_eyre::eyre::Result;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fmt::Write;
use std::io::{BufReader, Write as IoWrite};
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

#[derive(Debug, Deserialize)]
struct Root {
    packages: HashMap<String, PackageJson>,
}

#[derive(Debug, Deserialize)]
struct PackageJson {
    #[serde(default)]
    pname: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    system: Option<String>,
    #[serde(default, rename = "outputName")]
    output_name: Option<String>,
    #[serde(default)]
    meta: Option<MetaJson>,
}

#[derive(Debug, Default, Deserialize)]
struct MetaJson {
    #[serde(default)]
    available: Option<bool>,
    #[serde(default)]
    broken: Option<bool>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    homepage: Option<HomepageJson>,
    #[serde(default)]
    insecure: Option<bool>,
    #[serde(default)]
    unfree: Option<bool>,
    #[serde(default)]
    unsupported: Option<bool>,
    #[serde(default)]
    position: Option<String>,
    #[serde(default, rename = "longDescription")]
    long_description: Option<String>,
    #[serde(default, rename = "mainProgram")]
    main_program: Option<String>,
    #[serde(default)]
    license: Option<LicenseJson>,
    #[serde(default)]
    maintainers: Option<Vec<MaintainerJson>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum HomepageJson {
    Single(String),
    Multi(Vec<String>),
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LicenseJson {
    Object(LicenseObj),
    Array(Vec<LicenseObj>),
    String(String),
}

#[derive(Debug, Deserialize)]
struct LicenseObj {
    #[serde(default, rename = "spdxId")]
    spdx_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MaintainerJson {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    github: Option<String>,
    #[serde(default, rename = "githubId")]
    github_id: Option<i64>,
    #[serde(default)]
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

    let temp_path = crate::utils::get_data_dir().join("packages.json.br.tmp");
    let mut response = reqwest::get(&release.url).await?;
    let mut hasher = Sha256::new();
    {
        let mut file = std::fs::File::create(&temp_path)?;
        while let Some(chunk) = response.chunk().await? {
            hasher.update(&chunk);
            file.write_all(&chunk)?;
        }
    }

    let digest = hasher.finalize();
    let mut computed_hash = String::with_capacity(digest.len() * 2);
    for b in &digest {
        write!(&mut computed_hash, "{b:02x}")?;
    }

    if computed_hash != release.hash {
        let _ = std::fs::remove_file(&temp_path);
        return Err(color_eyre::eyre::eyre!(
            "Hash mismatch! Expected {}, got {}",
            release.hash,
            computed_hash
        ));
    }

    println!("Hash verified, decompressing and parsing...");
    let root: Root = {
        let file = std::fs::File::open(&temp_path)?;
        let buffered = BufReader::with_capacity(64 * 1024, file);
        let decoder = brotli::Decompressor::new(buffered, 64 * 1024);
        let json_reader = BufReader::with_capacity(64 * 1024, decoder);
        serde_json::from_reader(json_reader)?
    };
    let _ = std::fs::remove_file(&temp_path);

    let total = root.packages.len();
    println!("Creating database with {total} packages...");

    const BATCH_SIZE: usize = 5000;

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

    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "cache_size", "-64000")?;

    let mut count = 0;
    let mut package_batch: Vec<Package> = Vec::with_capacity(BATCH_SIZE);
    let mut maintainer_batch: Vec<Maintainer> = Vec::with_capacity(BATCH_SIZE * 4);

    for (pkg_name, pkg_data) in root.packages {
        let PackageJson {
            pname,
            version,
            name: display_name,
            system,
            output_name,
            meta,
        } = pkg_data;
        let m = meta.unwrap_or_default();

        let homepage = extract_homepage(m.homepage);
        let license_spdx = extract_license(m.license);

        if let Some(maints) = m.maintainers {
            for mt in maints {
                maintainer_batch.push(Maintainer {
                    package_name: pkg_name.clone(),
                    name: mt.name,
                    email: mt.email,
                    github: mt.github,
                    github_id: mt.github_id,
                    matrix: mt.matrix,
                });
            }
        }

        package_batch.push(Package {
            name: pkg_name,
            pname,
            version,
            display_name,
            system,
            output_name,
            available: i32::from(m.available.unwrap_or(false)),
            broken: i32::from(m.broken.unwrap_or(false)),
            description: m.description,
            homepage,
            insecure: i32::from(m.insecure.unwrap_or(false)),
            unfree: i32::from(m.unfree.unwrap_or(false)),
            unsupported: i32::from(m.unsupported.unwrap_or(false)),
            position: m.position,
            long_description: m.long_description,
            main_program: m.main_program,
            license_spdx_id: license_spdx,
            license_full_name: None,
            license_free: 0,
            license_url: None,
        });

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

fn extract_homepage(homepage: Option<HomepageJson>) -> Option<String> {
    match homepage? {
        HomepageJson::Single(s) => Some(s),
        HomepageJson::Multi(arr) => arr.into_iter().next(),
    }
}

fn extract_license(license: Option<LicenseJson>) -> Option<String> {
    match license? {
        LicenseJson::Object(o) => o.spdx_id,
        LicenseJson::Array(arr) => {
            let ids: Vec<String> = arr.into_iter().filter_map(|o| o.spdx_id).collect();
            if ids.is_empty() {
                None
            } else {
                Some(ids.join(", "))
            }
        }
        LicenseJson::String(s) => Some(s),
    }
}

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

fn print_progress(count: usize, total: usize) {
    #[allow(clippy::cast_precision_loss)]
    let progress = (count as f64 / total as f64) * 100.0;
    println!("Progress: {count}/{total} ({progress:.1}%)");
}
