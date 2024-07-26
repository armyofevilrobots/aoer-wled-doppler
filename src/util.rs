use crate::types::*;
use anyhow::{anyhow, Result};
use chrono::Datelike;
use fern::colors::{Color, ColoredLevelConfig};
use fern::log_file;
use log::{self, debug, error, info, trace, warn};
use mdns_sd::ServiceInfo;
use reqwest::Url;
use std::collections::HashMap;
use std::path::PathBuf;
use wled_json_api_library::structures::state::State;
use wled_json_api_library::wled::Wled;

pub fn configure_logging(loglevel: log::LevelFilter, logfile: Option<PathBuf>) {
    // Configure logger at runtime
    let colors = ColoredLevelConfig::new().debug(Color::Magenta);
    let fernlog = fern::Dispatch::new()
        .level(loglevel)
        .level_for("mdns_sd", log::LevelFilter::Warn)
        .level_for("reqwest", log::LevelFilter::Warn)
        .chain(
            fern::Dispatch::new()
                // Perform allocation-free log formatting
                .format(move |out, message, record| {
                    out.finish(format_args!(
                        "[{} {} {}] {}",
                        humantime::format_rfc3339_seconds(std::time::SystemTime::now()),
                        colors.color(record.level()),
                        record.target(),
                        message
                    ))
                })
                .chain(std::io::stdout()),
        );

    let fernlog = if let Some(logpath) = logfile {
        info!("Logging to '{:?}'", logpath);
        fernlog.chain(
            fern::Dispatch::new()
                // Perform allocation-free log formatting
                .format(move |out, message, record| {
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
                .level_for("reqwest", log::LevelFilter::Warn)
                .chain(
                    log_file(&logpath)
                        .expect(format!("Could not use log file path {:?}", &logpath).as_str()),
                ),
        )
    } else {
        fernlog
    };

    fernlog.apply().unwrap();
}

/// Set the brightness of the given wled device.
pub fn led_set_brightness(wled: &mut WLED, new_bri: u8) -> Result<()> {
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
    match wled.wled.flush_state() {
        Ok(response) => {
            trace!(
                "    - HTTP response: {:?}",
                response.text().unwrap_or("UNKNOWN ERROR".to_string())
            );
            Ok(())
        }
        Err(err) => {
            error!(
                "    - Failed to update WLED: '{}' with error: {:?}",
                &wled.name, err
            );
            Err(anyhow!(
                "Failed to update wled {} with error {:?}",
                &wled.name,
                err
            ))
        }
    }
}

pub fn update_wled_cache(info: &ServiceInfo, found_wled: &mut HashMap<String, WLED>) -> Result<()> {
    let full_name = info.get_fullname().to_string();
    let old_wled = found_wled.get(&full_name);
    if old_wled.is_none() || (!info.get_addresses().contains(&old_wled.unwrap().address)) {
        if old_wled.is_some() {
            warn!("WLED '{}' may have changed IPs. Updating.", full_name);
        }
        // let ip_addr: Option<IpAddr> = None;
        for try_ip in info.get_addresses() {
            let url: Url =
                Url::try_from(format!("http://{}:{}/", try_ip, info.get_port()).as_str())
                    .expect(format!("Invalid addr/port: {}:{}", try_ip, info.get_port()).as_str());
            info!("Found WLED at: {}", &url);
            let mut wled: Wled = Wled::try_from_url(&url).unwrap();
            // info!("new wled: {wled:?}");
            if let Ok(()) = wled.get_cfg_from_wled() {
                if let Some(cfg) = &wled.cfg {
                    // info!("WLED CFG: {:?}", &wled.cfg);
                    found_wled.insert(
                        full_name.to_string(),
                        WLED {
                            cfg: cfg.clone(),
                            address: try_ip.clone(),
                            port: info.get_port(),
                            name: info.get_fullname().to_string(),
                            wled: wled,
                        },
                    );
                    return Ok(());
                }
            }
        }
        return Err(anyhow!("Could not register WLED: {}", info.get_fullname()));
    }

    Ok(())
}

/// Calculates how much we should dim (from 0.0 as no dimming, to 1.0 as fully dimmed)
/// based on what time of day it is. Contains much magic (of the black datetime variety).
pub fn calc_dim_pc(
    today: chrono::DateTime<chrono::Local>,
    lat: f64,
    lon: f64,
    transition_duration: i64,
) -> f32 {
    // OK, Let's now calculate
    let today_date = today.date_naive();
    let (sunrise_time, sunset_time) = sunrise::sunrise_sunset(
        lat as f64,
        lon as f64,
        today_date.year(),
        today_date.month(),
        today_date.day(),
    );
    debug!("Sunrise, Sunset: {:?}, {:?}", sunrise_time, sunset_time);
    debug!(
        "Current unix time: {} and sunset is in {} seconds",
        today.timestamp(),
        sunset_time - today.timestamp()
    );
    if today.timestamp() > (sunrise_time + transition_duration)
        && today.timestamp() <= (sunset_time - transition_duration)
    {
        info!("No dim yet, still daytime");
        0.
    } else if today.timestamp() > (sunset_time - transition_duration)
        && today.timestamp() < sunset_time
    {
        info!("Twilight.");
        // OK, we're dimming
        (today.timestamp() - (sunset_time - transition_duration)) as f32
            / transition_duration as f32
    } else if today.timestamp() >= sunset_time {
        info!("MAX DIM; It's late.");
        1.
    } else if today.timestamp() <= sunrise_time {
        info!("MAX DIM; It's really fucking early.");
        1.
    } else if today.timestamp() > sunrise_time
        && today.timestamp() < sunrise_time + transition_duration
    {
        info!("Calc morning unDIM; It's early AM after sunrise.");
        1. - (today.timestamp() as f32 - sunrise_time as f32) / transition_duration as f32
    } else {
        // Fallback to super dim so we don't blind anybody if we escape those clausese^^
        1.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveDateTime, Utc};

    #[test]
    fn test_calc_dimming() {
        let today: chrono::DateTime<chrono::Local> = chrono::Local::now();
        let today_date = today.date_naive();
        let (sunrise_time, sunset_time) = sunrise::sunrise_sunset(
            49. as f64,
            -124. as f64,
            today_date.year(),
            today_date.month(),
            today_date.day(),
        );
        let dim_pc = calc_dim_pc(today, 49., -124., 1200);
        info!("DIM PC: {}", dim_pc);
        let sunset_dt = DateTime::from_timestamp(sunset_time + 0, 0).unwrap();
        let sunset_dt: DateTime<Local> = sunset_dt.into();

        let dim_pc = calc_dim_pc(sunset_dt, 49., -124., 1200);
        info!("DIM PC at sunset: {}", dim_pc);
    }

    #[test]
    fn test_bri_calc() {
        let high = 50u8;
        let low = 1u8;
        let gap = (high - low) as f32;
        let dim_pc = 1.0f32;
        let new_bri = (high as f32 - (dim_pc * gap)).min(255.).max(0.) as u8;
        info!("New bri is {}", new_bri);
    }
}
