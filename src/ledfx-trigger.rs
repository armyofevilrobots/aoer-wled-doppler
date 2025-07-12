use clap::Parser;
use inotify::{Inotify, WatchMask};
use log::{self, debug, info, warn};
use log::{error, trace};
use mdns_sd::{ServiceDaemon, ServiceEvent};
use opener;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIconEvent;
use util::led_set_preset;
// use wled_json_api_library::structures::state::State;
// use wled_json_api_library::wled::Wled;
mod config;
mod ledfx;
mod monitor;
mod systray;
mod types;
mod util;
use crate::config::{calc_actual_config_file, load_config};
use crate::ledfx::playpause;
use crate::types::*;
use crate::util::{calc_led_state_scheduled, led_set_brightness, update_wled_cache};

const SERVICE_NAME: &str = "_wled._tcp.local.";
// const NO_SCHEDULE: LEDScheduleSpec = LEDScheduleSpec::None;

fn main() {
    let args = Args::parse();

    let mut inotify = Inotify::init().expect("Failed to initialize inotify");
    let cfgfile = match args.config_path.clone() {
        Some(cfgpath) => cfgpath,
        None => calc_actual_config_file(None),
    };
    inotify
        .watches()
        .add(
            cfgfile,
            WatchMask::MODIFY | WatchMask::CREATE | WatchMask::DELETE,
        )
        .expect("Failed to add inotify watch");

    let mut svc_config = match load_config(args.config_path.clone()) {
        Ok(config) => config,
        Err(err) => {
            // panic!("Failed to load config: {:?}", err),
            eprintln!("Failed to load config: {:?}", err);
            std::process::exit(-1);
        }
    };

    let die_arc: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let die_arc_thread = die_arc.clone();
    let tray_svc_config = svc_config.clone();
    let mut ledfx_enabled: Arc<Mutex<bool>> = Arc::new(Mutex::new(true));
    let mut ledfx_enabled_moved = ledfx_enabled.clone();
    if svc_config.tray_icon {
        info!("Starting up tray icon...");
        let (config_msg, exit_msg, enabled_msg) = systray::launch_taskbar_icon();

        thread::spawn(move || loop {
            if let Ok(event) = TrayIconEvent::receiver().try_recv() {
                info!("tray event: {event:?}");
            }

            if let Ok(event) = MenuEvent::receiver().try_recv() {
                info!("Menu Event {event:?}");
                if let Ok(qid) = enabled_msg.lock() {
                    info!("Enabled ledfx is toggled.");
                    //ledfx_enabled = !ledfx_enabled;
                    {
                        let mut enabled = ledfx_enabled_moved
                            .lock()
                            .expect("Failed to lock enabled message");
                        *enabled = !*enabled;
                    }
                    //println!("Switched to: {}", &ledfx_enabled);
                }
                if let Ok(qid) = exit_msg.lock() {
                    if let Some(menuid) = qid.as_ref() {
                        if menuid == event.id() {
                            error!("Received QUIT from TaskBar icon!");
                            let mut die = die_arc_thread.lock().unwrap();
                            *die = true;
                            break;
                        }
                    }
                }
                if let Ok(cid) = config_msg.lock() {
                    if let Some(menuid) = cid.as_ref() {
                        if menuid == event.id() {
                            if let Some(urlbase) = &mut tray_svc_config.bind_address.clone() {
                                // Rewrite the URL if we bound everything.
                                *urlbase = urlbase.replace("0.0.0.0", "localhost");
                                info!("Launching browser...");
                                opener::open_browser(format!("http://{}/", urlbase))
                                    .unwrap_or_else(|_| warn!("Failed to launch browser."));
                            }
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_secs_f64(0.1));
        });
    }

    let mut next_ledfx_transition = svc_config.next_ledfx_transition();
    /*
    if let Some(ledfx_schedule) = svc_config.ledfx_schedule.clone() {
        let from_ts = ledfx_schedule
            .from
            .to_timestamp(svc_config.lat as f64, svc_config.lon as f64);
        let until_ts = ledfx_schedule
            .until
            .to_timestamp(svc_config.lat as f64, svc_config.lon as f64);
        if from_ts < until_ts {
            next_ledfx_transition = Some((ledfx_schedule.from.clone(), true));
        } else {
            next_ledfx_transition = Some((ledfx_schedule.until.clone(), false))
        }
    }
    */

    info!("==========================================================");
    info!("= ledfx-trigger booting...");
    info!("==========================================================");
    info!("Loaded config: {:?}", &svc_config);

    util::cfg_logging(svc_config.loglevel, svc_config.logfile.clone());
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    // let mut last_update = std::time::Instant::now();

    ///// Webserver
    // if let Some(server_bind) = &svc_config.bind_address {
    //     info!("Spawning webserver: {}", &server_bind);
    //     let cfg = svc_config.clone();
    //     thread::spawn(move || webui::spawn(cfg));
    // } else {
    //     info!("No bind config, not spawning webserver.");
    // }
    ///// /Webserver

    // OK, now we setup the monitoring...
    let (_stream, playing_arc) = if let Some(ref audio_config) = svc_config.audio_config {
        let (mon, playing_arc) = monitor::setup_audio(&audio_config).unwrap();
        (Some(mon), playing_arc)
    } else {
        (None, Arc::new(AtomicBool::new(false)))
    }; // Note: Stream has to stay in scope or it gets collected and audio dies.

    let mut quiet_cycles: usize = 0;
    let mut inotify_buffer = [0u8; 4096];
    let mut last_command_by_name: HashMap<String, (f32, Option<u16>, Option<bool>)> =
        HashMap::new();
    loop {
        loop {
            info!("Checking inotify events...");
            if let Ok(events) = inotify.read_events(&mut inotify_buffer) {
                let mut should_break: bool = false;
                for event in events {
                    info!("INOTIFY_EV: {:?}", event);
                    should_break = true;
                }
                if should_break {
                    break;
                }
            }
            // .read_events_blocking(&mut inotify_buffer)
            let now = std::time::Instant::now();
            if playing_arc.load(Relaxed) {
                debug!("arc says we are playing.");
                quiet_cycles = 0;
            } else {
                debug!("arc says we are quiet.");
                quiet_cycles =
                    (quiet_cycles + 1).min(&svc_config.ledfx_idle_cycles.unwrap_or(3) + 1);
            }
            if let Some(baseurl) = &svc_config.ledfx_url {
                let mut ledfx_enabled_locked = ledfx_enabled.lock().expect("Failed to unlock");
                debug!("Enabled is set to: {}", ledfx_enabled_locked);
                debug!("Got LEDFX url of {}", baseurl);
                // First, see if we toggle the state of it...
                if let Some((next_trigger, next_state)) = &next_ledfx_transition {
                    debug!(
                        "Got an enabled state change at {:?} to {:?}",
                        next_trigger, next_state
                    );
                    if next_trigger.to_timestamp(svc_config.lat as f64, svc_config.lon as f64)
                        < chrono::Local::now().timestamp() as u64
                    {
                        if let Some(enabled_state) = next_state {
                            *ledfx_enabled_locked = *enabled_state;
                        }
                        next_ledfx_transition = svc_config.next_ledfx_transition();
                    }
                } else {
                    debug!("NO LEDFX STATE TRIGGER SET");
                }
                if quiet_cycles >= svc_config.ledfx_idle_cycles.unwrap_or(3)
                    || !*ledfx_enabled_locked
                {
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

            let today: chrono::DateTime<chrono::Local> = chrono::Local::now();
            let mut leds_ok: usize = 0;
            let mut leds_noconfig: usize = 0;
            let leds_ignore: usize = 0;
            let mut leds_err: usize = 0;
            {
                // Locking die arc...
                let die = die_arc.lock().unwrap();
                if *die {
                    break;
                }
            } // Locking die arc...

            //std::thread::sleep(Duration::from_secs(10));
            std::thread::sleep(Duration::from_secs_f64(svc_config.cycle_seconds));
        } // Loop wleds
        match svc_config.restart_on_cfg_change {
            CfgChangeAction::No => (),
            CfgChangeAction::Exit => {
                info!("Exiting due to a config change.");
                std::process::exit(0);
            }
            CfgChangeAction::Reload => {
                info!("Reloading due to a config change.");
                let old_loglevel = svc_config.loglevel;
                let old_logfile = svc_config.logfile.clone();
                let old_tray_icon = svc_config.tray_icon;
                svc_config = match load_config(args.config_path.clone()) {
                    Ok(config) => config,
                    Err(err) => {
                        // panic!("Failed to load config: {:?}", err),
                        eprintln!("Failed to load config: {:?}", err);
                        std::process::exit(-1);
                    }
                };
                if old_loglevel != svc_config.loglevel
                    || old_logfile != svc_config.logfile
                    || old_tray_icon != svc_config.tray_icon
                {
                    warn!(
                        "Changes to logging and system tray configuration \
                           are ignored when restart_on_cfg_change is 'Reload'."
                    );
                }
            }
        }
        {
            // Locking die arc...
            let die = die_arc.lock().unwrap();
            if *die {
                break;
            }
        } // Locking die arc...
    } // Loop inotify
}
