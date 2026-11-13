use serde::{Deserialize, Serialize};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub server: Option<String>,
    pub space: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}

fn path() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("MATRICAL_CONFIG") {
        return Ok(path.into());
    }
    let home = std::env::var("HOME").map_err(|_| "HOME is not set".to_owned())?;
    Ok(PathBuf::from(home).join(".config/matrical/config.json"))
}

impl Config {
    pub fn load() -> Result<Self, String> {
        match std::fs::read(path()?) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let body = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)
            .map_err(|e| e.to_string())?;
        file.write_all(&body).map_err(|e| e.to_string())
    }
}
