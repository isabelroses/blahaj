use color_eyre::eyre::Result;
use serde::Deserialize;
use serde::de::{self, DeserializeSeed, Deserializer, IgnoredAny, MapAccess, Visitor};
use sha2::{Digest, Sha256};
use std::fmt::Write;
use std::io::{BufReader, Write as IoWrite};
use std::path::{Path, PathBuf};

use reqwest::Client;
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};

#[derive(Debug)]
struct NixpkgsRelease {
    url: String,
    hash: String,
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

async fn get_latest_nixpkgs_release(client: &Client) -> Result<NixpkgsRelease> {
    let base_url = &crate::config::get().nixpkgs_channel;

    let response = client.get(base_url).send().await?;
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
    let hash_path = crate::config::data_dir().join("nixpkgs.hash");
    std::fs::read_to_string(hash_path).ok()
}

fn store_hash(hash: &str) -> Result<()> {
    let hash_path = crate::config::data_dir().join("nixpkgs.hash");
    std::fs::write(hash_path, hash)?;
    Ok(())
}

/// Builds `packages.db` if it's missing, or rebuilds it if a newer nixpkgs
/// release is out.
pub async fn ensure_database(client: &Client) -> Result<()> {
    let data_dir = crate::config::data_dir();
    let db_path = data_dir.join("packages.db");

    println!("Checking for nixpkgs updates...");
    let release = get_latest_nixpkgs_release(client).await?;
    let stored_hash = get_stored_hash();

    if db_path.exists() && stored_hash.as_deref() == Some(&release.hash) {
        println!("nixpkgs database is up to date");
        return Ok(());
    }

    if db_path.exists() {
        println!("New nixpkgs release detected, updating database...");
    } else {
        println!("nixpkgs database not found, building...");
    }

    println!("Downloading from {}...", release.url);

    let temp_path = data_dir.join("packages.json.br.tmp");
    let mut response = client.get(&release.url).send().await?;
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

    println!("Hash verified, decompressing and building database...");

    // Several seconds of CPU: keep it off the async runtime's worker threads.
    let tmp_db_path = data_dir.join("packages.db.tmp");
    let (br, tmp) = (temp_path.clone(), tmp_db_path.clone());
    let build = tokio::task::spawn_blocking(move || build_database(&br, &tmp)).await?;
    let _ = std::fs::remove_file(&temp_path);
    if let Err(err) = build {
        let _ = std::fs::remove_file(&tmp_db_path);
        return Err(err);
    }

    // Atomic swap, so the database is never missing mid-rebuild.
    std::fs::rename(&tmp_db_path, &db_path)?;
    store_hash(&release.hash)?;

    println!("Database created successfully: {}", db_path.display());
    Ok(())
}

/// Builds a package database at `db_path` from the brotli-compressed
/// `packages.json` at `br_path`, streaming packages into it as they are parsed
/// rather than loading the whole map into memory.
fn build_database(br_path: &Path, db_path: &Path) -> Result<()> {
    let _ = std::fs::remove_file(db_path);
    let mut conn = Connection::open(db_path)?;

    // Throwaway until renamed into place, so skip crash safety.
    conn.pragma_update(None, "journal_mode", "OFF")?;
    conn.pragma_update(None, "synchronous", "OFF")?;
    conn.pragma_update(None, "cache_size", "-64000")?;

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

    let file = std::fs::File::open(br_path)?;
    let buffered = BufReader::with_capacity(64 * 1024, file);
    let decoder = brotli::Decompressor::new(buffered, 64 * 1024);
    let json_reader = BufReader::with_capacity(64 * 1024, decoder);

    let mut sink = PackageSink::new(&mut conn);
    let mut deserializer = serde_json::Deserializer::from_reader(json_reader);
    (&mut deserializer).deserialize_map(RootVisitor { sink: &mut sink })?;
    deserializer.end()?;
    let count = sink.finish()?;

    // Index after the bulk insert; `package_name` is already the primary key.
    conn.execute("CREATE INDEX idx_pname ON packages(pname)", [])?;
    conn.execute(
        "CREATE INDEX idx_maintainers_package ON maintainers(package_name)",
        [],
    )?;

    println!("Inserted {count} packages");
    Ok(())
}

const BATCH_SIZE: usize = 5000;

/// Collects parsed packages and writes them to the database in batches.
struct PackageSink<'c> {
    conn: &'c mut Connection,
    packages: Vec<Package>,
    maintainers: Vec<Maintainer>,
    count: usize,
}

impl<'c> PackageSink<'c> {
    fn new(conn: &'c mut Connection) -> Self {
        Self {
            conn,
            packages: Vec::with_capacity(BATCH_SIZE),
            maintainers: Vec::with_capacity(BATCH_SIZE * 4),
            count: 0,
        }
    }

    fn push(&mut self, pkg_name: String, pkg_data: PackageJson) -> Result<()> {
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
                self.maintainers.push(Maintainer {
                    package_name: pkg_name.clone(),
                    name: mt.name,
                    email: mt.email,
                    github: mt.github,
                    github_id: mt.github_id,
                    matrix: mt.matrix,
                });
            }
        }

        self.packages.push(Package {
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

        self.count += 1;

        if self.packages.len() >= BATCH_SIZE {
            self.flush()?;
            println!("Progress: {} packages", self.count);
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<()> {
        if !self.packages.is_empty() {
            insert_batch(self.conn, &self.packages, &self.maintainers)?;
            self.packages.clear();
            self.maintainers.clear();
        }
        Ok(())
    }

    fn finish(mut self) -> Result<usize> {
        self.flush()?;
        Ok(self.count)
    }
}

/// Visits the top-level `packages.json` object, streaming the `packages` map
/// into a [`PackageSink`] and skipping every other key.
struct RootVisitor<'a, 'c> {
    sink: &'a mut PackageSink<'c>,
}

impl<'de> Visitor<'de> for RootVisitor<'_, '_> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a packages.json object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while let Some(key) = map.next_key::<String>()? {
            if key == "packages" {
                map.next_value_seed(PackagesSeed {
                    sink: &mut *self.sink,
                })?;
            } else {
                map.next_value::<IgnoredAny>()?;
            }
        }
        Ok(())
    }
}

/// Deserialises the `packages` map one entry at a time into a [`PackageSink`].
struct PackagesSeed<'a, 'c> {
    sink: &'a mut PackageSink<'c>,
}

impl<'de> DeserializeSeed<'de> for PackagesSeed<'_, '_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for PackagesSeed<'_, '_> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("a map of package names to packages")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while let Some((name, pkg)) = map.next_entry::<String, PackageJson>()? {
            self.sink.push(name, pkg).map_err(de::Error::custom)?;
        }
        Ok(())
    }
}

/// Rebuilds `packages.db` every 12 hours when a new release is out.
pub fn spawn_refresher(client: Client) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_hours(12));
        // The first tick completes immediately, and startup already checked.
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(e) = ensure_database(&client).await {
                eprintln!("Failed to update nixpkgs database: {e}");
            }
        }
    });
}

fn db_path() -> PathBuf {
    crate::config::data_dir().join("packages.db")
}

/// A package as shown by `/nixpkgs`.
#[derive(Debug)]
pub struct PackageInfo {
    pub pname: String,
    pub version: String,
    pub description: String,
    pub homepage: Option<String>,
    pub license: String,
    /// `path/to/file.nix:line` within nixpkgs.
    pub position: String,
    pub broken: bool,
    pub insecure: bool,
    pub unfree: bool,
    pub maintainers: Vec<PackageMaintainer>,
}

#[derive(Debug)]
pub struct PackageMaintainer {
    pub name: String,
    pub github: Option<String>,
}

/// Looks up `package` by attribute name. Blocking.
///
/// Opens a fresh read-only connection each time, so lookups see the new file
/// as soon as a rebuild renames it into place.
pub fn lookup(package: &str) -> Result<Option<PackageInfo>> {
    let conn = Connection::open_with_flags(
        db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    Ok(lookup_in(&conn, package)?)
}

fn lookup_in(conn: &Connection, package: &str) -> rusqlite::Result<Option<PackageInfo>> {
    let Some(mut info) = conn
        .query_row(
            "SELECT pname, version, description, homepage, license_spdx_id,
                    position, broken, insecure, unfree
             FROM packages WHERE package_name = ?1",
            params![package],
            |row| {
                Ok(PackageInfo {
                    pname: row.get::<_, Option<String>>(0)?.unwrap_or_default(),
                    version: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    description: row.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    homepage: row.get(3)?,
                    license: row
                        .get::<_, Option<String>>(4)?
                        .unwrap_or_else(|| "Unknown".to_string()),
                    position: row
                        .get::<_, Option<String>>(5)?
                        .unwrap_or_else(|| "unknown".to_string()),
                    broken: row.get::<_, i32>(6)? != 0,
                    insecure: row.get::<_, i32>(7)? != 0,
                    unfree: row.get::<_, i32>(8)? != 0,
                    maintainers: Vec::new(),
                })
            },
        )
        .optional()?
    else {
        return Ok(None);
    };

    info.maintainers = conn
        .prepare("SELECT name, github FROM maintainers WHERE package_name = ?1")?
        .query_map(params![package], |row| {
            Ok(PackageMaintainer {
                name: row
                    .get::<_, Option<String>>(0)?
                    .unwrap_or_else(|| "Unknown".to_string()),
                github: row.get::<_, Option<String>>(1)?.filter(|g| !g.is_empty()),
            })
        })?
        .collect::<rusqlite::Result<_>>()?;

    Ok(Some(info))
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
    conn: &mut Connection,
    package_batch: &[Package],
    maintainer_batch: &[Maintainer],
) -> Result<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare_cached("INSERT INTO packages VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")?;
        for p in package_batch {
            stmt.execute(params![
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
            stmt.execute(params![
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_then_lookup_round_trip() {
        let dir = std::env::temp_dir().join(format!("blahaj-nixpkgs-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let br_path = dir.join("packages.json.br");
        let db_path = dir.join("packages.db");

        let json = r#"{
            "version": 2,
            "packages": {
                "hello": {
                    "pname": "hello",
                    "version": "2.12.3",
                    "meta": {
                        "description": "A program that produces a familiar, friendly greeting",
                        "homepage": ["https://www.gnu.org/software/hello/manual/"],
                        "license": [{"spdxId": "GPL-3.0-or-later"}],
                        "maintainers": [{"name": "Someone", "github": "someone"}]
                    }
                },
                "bare": {}
            },
            "trailing": {"ignored": [1, 2, 3]}
        }"#;
        {
            let file = std::fs::File::create(&br_path).unwrap();
            let mut writer = brotli::CompressorWriter::new(file, 4096, 5, 22);
            writer.write_all(json.as_bytes()).unwrap();
        }

        build_database(&br_path, &db_path).unwrap();

        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let count: i64 = conn
            .query_row("SELECT count(*) FROM packages", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);

        let (version, homepage, license): (String, String, String) = conn
            .query_row(
                "SELECT version, homepage, license_spdx_id FROM packages WHERE package_name = 'hello'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(version, "2.12.3");
        assert_eq!(homepage, "https://www.gnu.org/software/hello/manual/");
        assert_eq!(license, "GPL-3.0-or-later");

        let github: String = conn
            .query_row(
                "SELECT github FROM maintainers WHERE package_name = 'hello'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(github, "someone");

        let hello = lookup_in(&conn, "hello").unwrap().unwrap();
        assert_eq!(hello.pname, "hello");
        assert_eq!(hello.license, "GPL-3.0-or-later");
        assert_eq!(hello.maintainers[0].github.as_deref(), Some("someone"));
        let bare = lookup_in(&conn, "bare").unwrap().unwrap();
        assert_eq!(bare.license, "Unknown");
        assert!(lookup_in(&conn, "missing").unwrap().is_none());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
