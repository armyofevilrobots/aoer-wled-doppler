use anyhow::{anyhow, Result};
use fern::colors::{Color, ColoredLevelConfig};
use log::{self, debug, error, info, trace, warn, SetLoggerError};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::collections::HashMap;
use std::time::Duration;
use wled_json_api_library::structures::state::State;
mod types;
mod util;
mod config;
use types::*;
use util::*;
use config::*;

const SERVICE_NAME: &'static str = "_wled._tcp.local.";

fn main() {
    let mut found_wled: HashMap<String, WLED> = HashMap::new();
    /*
    let svc_config: Config = Config {
        lat: 49.4,
        lon: -123.7,
        exclusions: Vec::new(),
        brightnesses: HashMap::from([
            ("wled-vu-strip._wled._tcp.local.".to_string(), (1, 10)),
            ("wled-derek-matrix-1._wled._tcp.local.".to_string(), (1, 5)),
            ("wled-derek-desk._wled._tcp.local.".to_string(), (1, 30)),
        ]), //HashMap::new(),
        transition_duration: 3600i64,
        loglevel: 4,
    };*/
    let svc_config = match load_config(){
        Ok(config) => config,
        Err(err) => panic!("Failed to load config: {:?}", err),
    };
    
    let levels = vec![
        log::LevelFilter::Off,
        log::LevelFilter::Error,
        log::LevelFilter::Warn,
        log::LevelFilter::Info,
        log::LevelFilter::Debug,
    ];
    configure_logging(
        *levels
            .get(svc_config.loglevel)
            .unwrap_or(&log::LevelFilter::Info),
        svc_config.logfile
    );

    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    let receiver = mdns.browse(SERVICE_NAME).expect("Failed to browse");
    let mut last_update = std::time::Instant::now();
    loop {
        let now = std::time::Instant::now();
        while !receiver.is_empty() {
            if let Ok(event) = receiver.recv() {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        let _ = update_wled_cache(&info, &mut found_wled); // TODO: Fix this<--
                    }
                    other_event => {
                        trace!("At {:?} : {:?}", now.elapsed(), &other_event);
                    }
                }
            }
        }

        let today: chrono::DateTime<chrono::Local> = chrono::Local::now();
        let dim_pc: f32 = calc_dim_pc(
            today,
            svc_config.lat as f64,
            svc_config.lon as f64,
            svc_config.transition_duration,
        );
        debug!("Dim out is: {}", dim_pc);
        let mut leds_ok: usize = 0;
        for (name, wled) in found_wled.iter_mut() {
            if let Some((low, high)) = svc_config.brightnesses.get(name) {
                let gap = (high - low) as f32;
                let new_bri = (*high as f32 - (dim_pc * gap)).min(255.).max(0.) as u8;
                debug!(
                    " - Found WLED to dim: {} -> ({},{}) to {}",
                    name, low, high, new_bri
                );
                let result = led_set_brightness(wled, new_bri);
                if result.is_ok() {
                    leds_ok += 1;
                }
            }
        }
        if leds_ok == found_wled.len() {
            info!("{} out of {} WLEDs updated OK.", leds_ok, found_wled.len());
        } else {
            warn!("{} out of {} WLEDs updated OK.", leds_ok, found_wled.len());
        }

        std::thread::sleep(Duration::from_secs(10));
    }
}
