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
mod webui;
use crate::config::{calc_actual_config_file, load_config};
use crate::ledfx::playpause;
use crate::types::*;
use crate::util::{calc_led_state_scheduled, led_set_brightness, update_wled_cache};

const SERVICE_NAME: &str = "_wled._tcp.local.";
// const NO_SCHEDULE: LEDScheduleSpec = LEDScheduleSpec::None;

fn main() {
    let mut found_wled: HashMap<String, WLED> = HashMap::new();

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
    if svc_config.tray_icon {
        info!("Starting up tray icon...");
        let (config_msg, exit_msg) = systray::launch_taskbar_icon();

        thread::spawn(move || loop {
            if let Ok(event) = TrayIconEvent::receiver().try_recv() {
                println!("tray event: {event:?}");
            }

            if let Ok(event) = MenuEvent::receiver().try_recv() {
                println!("tray event: {event:?}");
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
                            info!("Launching browser...");
                            opener::open_browser("https://armyofevilrobots.com/")
                                .unwrap_or_else(|_|warn!("Failed to launch browser."));
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_secs_f64(0.1));
        });
    }

    info!("==========================================================");
    info!("= aoer-wled-doppler starting. Scanning for WLED devices...");
    info!("==========================================================");
    info!("Loaded config: {:?}", &svc_config);

    util::cfg_logging(svc_config.loglevel, svc_config.logfile.clone());
    let mdns = ServiceDaemon::new().expect("Failed to create daemon");
    let receiver = mdns.browse(SERVICE_NAME).expect("Failed to browse");
    // let mut last_update = std::time::Instant::now();


    ///// Webserver
    if let Some(server_bind) = &svc_config.bind_address{
        info!("Spawning webserver: {}", &server_bind);
        let cfg = svc_config.clone();
        thread::spawn(move||webui::spawn(cfg));
        
    }else{
        info!("No bind config, not spawning webserver.");
    }
    ///// /Webserver

    // OK, now we setup the monitoring...
    let (_stream, playing_arc) = if let Some(audio_config) = svc_config.audio_config {
        let (mon, playing_arc) = monitor::setup_audio(&audio_config).unwrap();
        (Some(mon), playing_arc)
    } else {
        (None, Arc::new(AtomicBool::new(false)))
    }; // Note: Stream has to stay in scope or it gets collected and audio dies.

    let mut quiet_cycles: usize = 0;
    let mut inotify_buffer = [0u8; 4096];
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
            let mut leds_ok: usize = 0;
            let mut leds_noconfig: usize = 0;
            let leds_ignore: usize = 0;
            let mut leds_err: usize = 0;
            debug!(
                "Found devices: {:?}",
                found_wled.keys().cloned().collect::<Vec<String>>()
            );

            for (name, wled) in found_wled.iter_mut() {
                debug!("Processing {}", name);
                if let Some(led_bri_config) = svc_config.leds.get(name) {
                    debug!("Found BRI config for LED: {}", name);
                    if !matches!(led_bri_config.schedule, LEDScheduleSpec::None) {
                        let schedule_name = match led_bri_config.schedule.clone() {
                            LEDScheduleSpec::Default => "default".to_string(),
                            LEDScheduleSpec::ByName(foo) => foo.clone(),
                            _ => panic!("How TF did we get past the guard?!"),
                        };
                        debug!("Found valid schedule spec for LED: {}", name);
                        // Determine which schedule to use for this LED
                        if let Some(led_schedule) = svc_config.schedule.get(&schedule_name) {
                            // Yay, a match. figure out the dimming.
                            debug!("Found a working schedule for spec: {:?}", &led_schedule);
                            let (dim_pc, preset_id) = calc_led_state_scheduled(
                                today,
                                svc_config.lat as f64,
                                svc_config.lon as f64,
                                led_schedule,
                            );
                            debug!("Got dim/preset result of ({:?}, {:?}) for {}",
                                   &dim_pc, &preset_id, &name);

                            if let Some(id) = preset_id {
                                debug!("Setting LED {} to preset {}", name, id);
                                if let Ok(_) = wled.wled.get_state_from_wled() {
                                    if let Some(state) = &wled.wled.state {
                                        if state.ps != Some(id as i32) {
                                            match led_set_preset(wled, id) {
                                                Ok(_) => (),
                                                Err(err) => error!(
                                                    "Failed to set {} preset to {} : {:?}",
                                                    name, id, err
                                                ),
                                            }
                                        } else {
                                            debug!(
                                                "Found matching preset state already for {}:{}",
                                                name, id
                                            );
                                        }
                                    }else{
                                        warn!("LED {} has no state data.", name);
                                    }
                                }else{
                                    error!("Failed to get current LED state for {}.",name);
                                }
                            }else{
                                debug!("No preset ID set for wled {} now.", name)
                            }

                            debug!("Dim_PC is {}", dim_pc);

                            let gap =
                                ((led_bri_config.max_bri - led_bri_config.min_bri) as f32).abs();
                            let out = ((gap * dim_pc) + led_bri_config.min_bri as f32)
                                .max(0.)
                                .min(255.) as u8;
                            debug!(
                                "Setting LED:{} to BRI{} (min:{},max:{},dim:{})",
                                name, out, led_bri_config.min_bri, led_bri_config.max_bri, dim_pc
                            );
                            let result = led_set_brightness(wled, out);
                            if result.is_err() {
                                error!(
                                    "Failed to update LED:{} with err:{:?}",
                                    name,
                                    result.err().unwrap()
                                );
                                leds_err += 1;
                            } else {
                                leds_ok += 1;
                            }
                        } else {
                            panic!(
                                "Could not find brightness schedule {} in config for LED {}",
                                &schedule_name, &name
                            );
                        }; // Got the schedule
                    } else {
                        warn!("LED: {} configured for no management.", name);
                    }; // if matches(led_config_None)

                // leds_ok += 1;
                } else {
                    leds_noconfig += 1;
                } // if let some(bri_wled_config)
            } // for (name,wled) in found wleds.
            if leds_ok == found_wled.len() {
                info!(
                    "({}:OK, {}:IGNORE, {}:NOCFG, {}:ERR) out of {} WLEDs processed.",
                    leds_ok,
                    leds_ignore,
                    leds_noconfig,
                    leds_err,
                    found_wled.len()
                );
            } else {
                warn!(
                    "({}:OK, {}:IGNORE, {}:NOCFG, {}:ERR) out of {} WLEDs processed.",
                    leds_ok,
                    leds_ignore,
                    leds_noconfig,
                    leds_err,
                    found_wled.len()
                );
            }
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
