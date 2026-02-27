use directories::ProjectDirs;
use jukebox::AudioRating;
use serde::{Deserialize, Serialize};

const CONFIG_NAME: &str = "settings.toml";

#[derive(Serialize, Deserialize)]
pub struct Settings {
    pub skip_rating: AudioRating,
}

// TODO: Add version?
// TODO: Add default/reset/discard changes.

impl Settings {
    pub fn read() -> Result<Self, Box<dyn std::error::Error>> {
        let Some(proj_dirs) = ProjectDirs::from("org", "hikikones", crate::APP_NAME) else {
            return Ok(Self::default());
        };

        let config_path = proj_dirs.config_dir().join(CONFIG_NAME);

        match std::fs::read(config_path) {
            Ok(bytes) => Ok(toml::from_slice(&bytes)?),
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => Ok(Self::default()),
                _ => Err(err)?,
            },
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(proj_dirs) = ProjectDirs::from("org", "hikikones", crate::APP_NAME) else {
            return Err(
                "Unable to save settings due to no valid home directory path could be retrieved from the operating system",
            )?;
        };

        let config_path = proj_dirs.config_dir().join(CONFIG_NAME);

        let toml = toml::to_string(self)?;
        std::fs::write(config_path, toml)?;
        Ok(())
    }

    pub fn hash(&self) -> u64 {
        toml::to_string(self)
            .map(|s| seahash::hash(s.as_bytes()))
            .unwrap_or(0)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            skip_rating: Default::default(),
        }
    }
}
