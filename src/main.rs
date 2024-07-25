use log::{self, debug, error, info, warn, SetLoggerError};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use std::collections::HashMap;
use std::time::Duration;
use wled_json_api_library::structures::state::State;
mod types;
mod util;
use types::*;
use util::*;

const SERVICE_NAME: &'static str = "_wled._tcp.local.";

fn configure_logging(loglevel: log::LevelFilter) {
    // Configure logger at runtime
    fern::Dispatch::new()
        // Perform allocation-free log formatting
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                humantime::format_rfc3339(std::time::SystemTime::now()),
                record.level(),
                record.target(),
                message
            ))
        })
        .level(loglevel)
        .level_for("mdns_sd", log::LevelFilter::Warn)
        .chain(std::io::stdout())
        .apply()
        .unwrap();
}

fn main() {
    configure_logging(log::LevelFilter::Debug);
    let mut found_wled: HashMap<String, WLED> = HashMap::new();
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
    };

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
                        warn!("At {:?} : {:?}", now.elapsed(), &other_event);
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
        for (name, wled) in found_wled.iter_mut() {
            if let Some((low, high)) = svc_config.brightnesses.get(name) {
                let gap = (high - low) as f32;
                let new_bri = (*high as f32 - (dim_pc * gap)).min(255.).max(0.) as u8;
                debug!(" - Found WLED to dim: {} -> ({},{}) to {}", name, low, high, new_bri);

                wled.wled.state = Some(State {
                    on: Some(true),
                    bri: Some(new_bri),
                    transition: None,
                    tt: None,
                    ps: None,
                    psave: None,
                    pl: None,
                    nl: None,
                    udpn: None,
                    v: None,
                    rb: None,
                    live: None,
                    lor: None,
                    time: None,
                    mainseg: None,
                    playlist: None,
                    seg: None,
                });
                if let Ok(response) = wled.wled.flush_state() {
                    debug!(
                        "    - HTTP response: {:?}",
                        response.text().unwrap_or("UNKNOWN ERROR".to_string())
                    );
                }
            }
        }

        std::thread::sleep(Duration::from_secs(10));
    }
}
