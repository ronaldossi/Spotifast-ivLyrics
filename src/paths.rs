//! Where Spotifast keeps its files.
//!
//! Configuration, durable non-secret state, and disposable caches live in the
//! platform's conventional directories. Spotify grants use the platform store;
//! the token paths below are retained only for migration and sign-out cleanup.

use std::path::PathBuf;

use directories::ProjectDirs;

#[derive(Clone, Debug)]
pub struct AppDirs {
    pub config: PathBuf,
    pub state: PathBuf,
    pub cache: PathBuf,
}

impl AppDirs {
    pub fn discover() -> Self {
        Self::for_name("spotifast")
    }

    fn for_name(name: &str) -> Self {
        let organization = if cfg!(windows) { "" } else { "paolino" };
        let project = ProjectDirs::from("me", organization, name);
        match project {
            Some(project) => Self {
                config: project.config_dir().to_path_buf(),
                state: project
                    .state_dir()
                    .map(|path| path.to_path_buf())
                    .unwrap_or_else(|| project.data_local_dir().to_path_buf()),
                cache: project.cache_dir().to_path_buf(),
            },
            None => {
                let fallback = std::env::current_dir().unwrap_or_default();
                Self {
                    config: fallback.join(format!("{name}-config")),
                    state: fallback.join(format!("{name}-state")),
                    cache: fallback.join(format!("{name}-cache")),
                }
            }
        }
    }

    #[cfg(windows)]
    pub(crate) fn legacy() -> Option<Self> {
        ProjectDirs::from("me", "paolino", "spotifast").map(|project| Self {
            config: project.config_dir().to_path_buf(),
            state: project.data_local_dir().to_path_buf(),
            cache: project.cache_dir().to_path_buf(),
        })
    }

    /// Called before claiming the new instance slot, after checking the old one.
    #[cfg(windows)]
    pub fn migrate_legacy(&self) -> std::io::Result<()> {
        let Some(old) = Self::legacy() else {
            return Ok(());
        };
        for (source, destination) in [
            (&old.config, &self.config),
            (&old.state, &self.state),
            (&old.cache, &self.cache),
        ] {
            move_profile_directory(source, destination)?;
        }
        for source in [&old.config, &old.state, &old.cache] {
            if let Some(parent) = source.parent() {
                let _ = std::fs::remove_dir(parent);
                if let Some(organization) = parent.parent() {
                    let _ = std::fs::remove_dir(organization);
                }
            }
        }
        Ok(())
    }

    pub fn settings_file(&self) -> PathBuf {
        self.config.join("settings.json")
    }

    /// Winamp skins the listener has added, as `.wsz` files or folders.
    pub fn skins_dir(&self) -> PathBuf {
        self.config.join("skins")
    }

    pub fn session_file(&self) -> PathBuf {
        self.state.join("session.json")
    }

    /// What was played here, which Spotify never hears about and so
    /// cannot tell us later. See [`crate::history`].
    pub fn history_file(&self) -> PathBuf {
        self.state.join("history.json")
    }

    pub fn shared_web_token_file(&self) -> PathBuf {
        self.state.join("shared_web_api_token.json")
    }

    pub fn personal_web_token_file(&self) -> PathBuf {
        self.state.join("personal_web_api_token.json")
    }

    pub fn legacy_web_token_file(&self) -> PathBuf {
        self.state.join("web_api_token.json")
    }

    /// The log of the current run, replaced at every start.
    pub fn log_file(&self) -> PathBuf {
        self.state.join("spotifast.log")
    }

    /// Where a panic is recorded before the process dies of it.
    pub fn panic_log(&self) -> PathBuf {
        self.state.join("panic.log")
    }

    pub fn credentials_dir(&self) -> PathBuf {
        self.state.join("credentials")
    }

    /// Optional proxy password, owner-only, never written to settings.json.
    pub fn proxy_secret_file(&self) -> PathBuf {
        self.state.join("proxy_password")
    }

    pub fn volume_dir(&self) -> PathBuf {
        self.state.join("volume")
    }

    pub fn audio_cache_dir(&self) -> PathBuf {
        self.cache.join("audio")
    }

    pub fn art_cache_dir(&self) -> PathBuf {
        self.cache.join("art")
    }

    pub fn lyrics_cache_dir(&self) -> PathBuf {
        self.cache.join("lyrics")
    }

    pub fn playlist_cache_dir(&self) -> PathBuf {
        self.cache.join("playlists")
    }

    pub fn account_playlist_cache_dir(&self, account_id: &str) -> PathBuf {
        self.playlist_cache_dir().join(account_id)
    }

    pub fn liked_songs_cache_file(&self, account_id: &str) -> PathBuf {
        // Hex encoding also keeps unusual account IDs within the cache root.
        let account: String = account_id
            .bytes()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        self.cache
            .join("liked-songs")
            .join(format!("{account}.json"))
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        for dir in [&self.config, &self.state, &self.cache] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

#[cfg(any(windows, test))]
fn move_profile_directory(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> std::io::Result<()> {
    if !source.exists() {
        return Ok(());
    }
    if !destination.exists() {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        return std::fs::rename(source, destination);
    }
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            move_profile_directory(&entry.path(), &target)?;
        } else if !target.exists() {
            std::fs::rename(entry.path(), target)?;
        }
    }
    // Existing destination files win; conflicting originals remain recoverable.
    match std::fs::remove_dir(source) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::DirectoryNotEmpty => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_migration_preserves_conflicts_and_moves_all_other_data() {
        let root = std::env::temp_dir().join(format!(
            "spotifast-profile-migration-{}",
            std::process::id()
        ));
        let old = root.join("paolino/spotifast");
        let new = root.join("spotifast");
        std::fs::create_dir_all(old.join("cache")).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(old.join("settings.json"), "old settings").unwrap();
        std::fs::write(new.join("settings.json"), "new settings").unwrap();
        std::fs::write(old.join("cache/art"), "artwork").unwrap();
        move_profile_directory(&old, &new).unwrap();
        assert_eq!(
            std::fs::read_to_string(new.join("settings.json")).unwrap(),
            "new settings"
        );
        assert_eq!(
            std::fs::read_to_string(old.join("settings.json")).unwrap(),
            "old settings"
        );
        assert_eq!(
            std::fs::read_to_string(new.join("cache/art")).unwrap(),
            "artwork"
        );
        move_profile_directory(&old, &new).unwrap();
        std::fs::remove_file(old.join("settings.json")).unwrap();
        move_profile_directory(&old, &new).unwrap();
        assert!(!old.exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
