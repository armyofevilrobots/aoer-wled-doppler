use std::{collections::HashMap, path::PathBuf};
use std::net::IpAddr;
use wled_json_api_library::wled::Wled;
use wled_json_api_library::structures::state::State;
use serde::{Serialize, Deserialize};
use clap::Parser;


/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub(crate) struct Args {
    /// Name of the person to greet
    #[arg(short, long)]
    pub config_path: Option<PathBuf>,

    // /// Number of times to greet
    // #[arg(short, long, default_value_t = 1)]
    // count: u8,
}


#[derive(Debug)]
pub struct WLED {
    pub state: State,
    pub address: IpAddr,
    pub name: String,
    pub wled: Wled,
}


#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct AudioConfig{
    #[serde(default = "default_input_device")]
    pub input_device: String,
    #[serde(default = "default_jack")]
    pub jack: bool,
    pub ledfx_threshold_db: Option<f32>
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ScheduleTime{
    Sunrise,
    SunriseOffset(i16),
    Sunset,
    SunsetOffset(i16),
    Time(chrono::NaiveTime),
}

// These are used as a list of times with intensities 0-255.
// We interpolate linearly, and treat each day as a loop, so
// we interpolate between the last time in the previous day,
// and the first time today.
#[derive(Debug, Serialize, Deserialize)]
pub struct DayNightPresets{
    brightness: u8,
    time: ScheduleTime,
}


fn default_input_device()->String{
    "default".to_string()
}

fn default_jack()->bool{
    false
}

fn default_cycle()->f64{
    10.0
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
    pub audio_config: Option<AudioConfig>,
    pub ledfx_url: Option<String>,
    pub ledfx_idle_cycles: Option<usize>,
    #[serde(default = "default_cycle")]
    pub cycle_seconds: f64,
}

fn default_logfile()->Option<PathBuf>{
    None
}
