use directories::ProjectDirs;
use jukebox::AudioRating;
use serde::{Deserialize, Serialize};

const CONFIG_NAME: &str = "settings.toml";

#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub skip_rating: AudioRating,
}

// TODO: Add version?

impl Settings {
    pub fn read() -> Result<Self, Box<dyn std::error::Error>> {
        let Some(proj_dirs) = ProjectDirs::from("org", "hikikones", crate::APP_NAME) else {
            return Ok(Self::default());
        };

        let config_path = proj_dirs.config_dir().join(CONFIG_NAME);

        match std::fs::read(&config_path) {
            Ok(bytes) => {
                let toml: Self = toml::from_slice(&bytes).map_err(|err| {
                    format!(
                        "Failed to deserialize settings from \"{}\" due to {}",
                        config_path.display(),
                        err
                    )
                })?;
                Ok(toml)
            }
            Err(err) => match err.kind() {
                std::io::ErrorKind::NotFound => Ok(Self::default()),
                _ => Err(format!(
                    "Failed to read settings from \"{}\" due to {}",
                    config_path.display(),
                    err
                ))?,
            },
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let Some(proj_dirs) = ProjectDirs::from("org", "hikikones", crate::APP_NAME) else {
            return Err(
                "Unable to save settings due to no valid home directory path that could be retrieved from the operating system",
            )?;
        };

        let dir = proj_dirs.config_dir();
        if !dir.exists() {
            std::fs::create_dir_all(dir).map_err(|err| {
                format!(
                    "Failed to create directory \"{}\" due to {}",
                    dir.display(),
                    err
                )
            })?;
        }

        let file = dir.join(CONFIG_NAME);
        let toml = toml::to_string(self)
            .map_err(|err| format!("Failed to serialize settings due to {}", err))?;
        std::fs::write(file, toml).map_err(|err| {
            format!(
                "Failed to write settings to \"{}\" due to {}",
                dir.display(),
                err
            )
        })?;

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
