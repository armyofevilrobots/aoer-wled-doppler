use std::{collections::HashMap, path::PathBuf};
use std::net::IpAddr;
use wled_json_api_library::wled::Wled;
use wled_json_api_library::structures::cfg::Cfg;
use serde::{Serialize, Deserialize};

#[derive(Debug)]
pub struct WLED {
    pub cfg: Cfg,
    pub address: IpAddr,
    pub port: u16,
    pub name: String,
    pub wled: Wled,
}

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub lat: f32,
    pub lon: f32,
    pub exclusions: Vec<String>,
    pub brightnesses: HashMap<String, (u8, u8)>,
    pub transition_duration: i64, // How long it takes to go full dim from full bright
    pub loglevel: usize, //0: off, 1: error, 2: warn, 3: info, 4: debug, 5: pedantic
    #[serde(default = "default_logfile")]
    pub logfile: Option<PathBuf>,
}

fn default_logfile()->Option<PathBuf>{
    None
}
