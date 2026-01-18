use std::path::PathBuf;

/// Get the data directory for application files following XDG standards
pub fn get_data_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "blahaj")
        .map_or_else(
            || PathBuf::from("/var/lib/blahaj"),
            |dirs| dirs.data_dir().to_path_buf(),
        )
}
