use log::trace;
use log::{self, debug, info, warn};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Arc;
use std::time::Duration;
use wled_json_api_library::structures::cfg::cfg_def::Def;
mod config;
mod ledfx;
mod monitor;
mod types;
mod util;
use config::*;
use ledfx::playpause;
use types::*;
use util::*;

const SERVICE_NAME: &'static str = "_wled._tcp.local.";

fn main() {
    let mut found_wled: HashMap<String, WLED> = HashMap::new();
    let svc_config = match load_config() {
        Ok(config) => config,
        Err(err) => panic!("Failed to load config: {:?}", err),
    };

    let levels = vec![
        log::LevelFilter::Off,
        log::LevelFilter::Error,
        log::LevelFilter::Warn,
        log::LevelFilter::Info,
        log::LevelFilter::Debug,
        log::LevelFilter::Trace,
    ];
    configure_logging(
        *levels
            .get(svc_config.loglevel)
            .unwrap_or(&log::LevelFilter::Info),
        svc_config.logfile.clone(),
    );
    info!("==========================================================");
    info!("= aoer-wled-doppler starting. Scanning for WLED devices...");
    info!("==========================================================");
    info!("Loaded config: {:?}", &svc_config);

    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    let receiver = mdns.browse(SERVICE_NAME).expect("Failed to browse");
    // let mut last_update = std::time::Instant::now();

    // OK, now we setup the monitoring...
    let (_stream, playing_arc) = if let Some(audio_config) = svc_config.audio_config {
        let (mon, playing_arc) = monitor::setup_audio(&audio_config).unwrap();
        (Some(mon), playing_arc)
    } else {
        (None, Arc::new(AtomicBool::new(false)))
    }; // Note: Stream has to stay in scope or it gets collected and audio dies.

    let mut quiet_cycles: usize = 0;
    loop {
        let now = std::time::Instant::now();
        if playing_arc.load(Relaxed) {
            debug!("arc says we are playing.");
            quiet_cycles = 0;
        } else {
            debug!("arc says we are quiet.");
            quiet_cycles = (quiet_cycles + 1).max(&svc_config.ledfx_idle_cycles.unwrap_or(3) + 1);
        }

        if let Some(baseurl) = &svc_config.ledfx_url {
            debug!("Got LEDFX url of {}", baseurl);
            if quiet_cycles >= svc_config.ledfx_idle_cycles.unwrap_or(3) {
                // Again, arbitrary
                debug!("We have been quiet for a couple cycles.");
                playpause(baseurl.as_str(), true).unwrap_or_else(|_| {
                    warn!("Failed to pause LEDFX!");
                });
            } else {
                debug!("We have NOT been quiet for a couple cycles. Showing LEDFX.");
                playpause(baseurl.as_str(), false).unwrap_or_else(|_| {
                    warn!("Failed to pause LEDFX!");
                })
            }
        } else {
            debug!("No LEDFX url found. Skipping updates.");
        }
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
        debug!("Dim out is: {}%", (dim_pc * 100.) as usize);
        let mut leds_ok: usize = 0;
        for (name, wled) in found_wled.iter_mut() {
            let new_bri = if let Some((low, high)) = svc_config.brightnesses.get(name) {
                let gap = (high - low) as f32;
                (*high as f32 - (dim_pc * gap)).min(255.).max(0.) as u8
            } else if !svc_config.exclusions.contains(&name) {
                let default_bri = &wled
                    .cfg
                    .clone()
                    .def
                    .unwrap_or(Def::default())
                    .bri
                    .unwrap_or(128);
                let (high, low) = (*default_bri as f32, *default_bri as f32 / 4.);
                info!(
                    "WLED {} being dimmed to default {}-{} range",
                    &name,
                    low.max(255.) as u8,
                    high.min(0.) as u8
                );
                let gap = (high - low) as f32;
                (high as f32 - (dim_pc * gap)).min(255.).max(0.) as u8
            } else {
                info!("WLED {} is excluded. Not dimming.", &name);
                leds_ok += 1;
                0 // Required else has no dimming. Or else ;)
            };

            if !svc_config.exclusions.contains(&name) {
                debug!("Dimming LED {} to {}", &name, new_bri);
                let result = led_set_brightness(wled, new_bri);
                if result.is_ok() {
                    leds_ok += 1;
                }
            }
        }
        if leds_ok == found_wled.len() {
            info!(
                "{} out of {} WLEDs processed OK.",
                leds_ok,
                found_wled.len()
            );
        } else {
            warn!(
                "{} out of {} WLEDs processed OK.",
                leds_ok,
                found_wled.len()
            );
        }

        std::thread::sleep(Duration::from_secs(10));
    }
}
