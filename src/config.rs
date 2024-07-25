use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

/// Manages the configuration; related tools.
use crate::types::*;
use anyhow::{anyhow, Result};

fn calc_config_dir() -> PathBuf {
    let mut homedir = dirs::home_dir().expect("Must have a $HOME dir set to run.");
    homedir.push(".wled-doppler");
    homedir
}

fn bootstrap() -> Result<PathBuf> {
    let homedir = calc_config_dir();
    if homedir.is_file() {
        panic!(
            "Configuration dir is a file instead of a dir. {:?}",
            &homedir
        );
    }
    if !homedir.exists() {
        std::fs::create_dir_all(&homedir)?
    }

    let mut cfgpath = homedir.clone();
    cfgpath.push("config.ron");
    if !cfgpath.is_file() {
        let tmpconfig = Config {
            lat: 49.0,
            lon: -124.0,
            exclusions: Vec::new(),
            brightnesses: HashMap::new(),
            transition_duration: 3600i64,
            loglevel: 4,
        };
        let mut cfgstr = ron::ser::to_string_pretty(&tmpconfig, ron::ser::PrettyConfig::default())
            .expect("Wups, my default config is borked?!");
        // let mut cfgfile = std::fs::File::create(&cfgpath)?;
        // cfgfile.write_all(cfgstr.as_bytes())?;
        // cfgfile.flush()?;
        std::fs::write(&cfgpath, cfgstr.as_bytes())?
    }

    Ok(cfgpath)
}

pub fn load_config() -> Result<Config> {
    let cfgdir = bootstrap()?;
    let cfgfile = std::fs::read_to_string(cfgdir)?;
    let cfg: Config = ron::de::from_bytes(cfgfile.as_bytes())?;
    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, Utc};

    #[test]
    fn test_setup_homedir() {
        let cfg = load_config().expect("Failed to load config from file.");
        assert!(cfg.loglevel == 4);
    }
}
