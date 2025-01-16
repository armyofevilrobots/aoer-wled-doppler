use std::time::Duration;
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIconEvent;

mod config;
mod ledfx;
mod monitor;
mod systray;
mod types;
mod util;
mod ui;

const SERVICE_NAME: &str = "_wled._tcp.local.";


fn main() {
    let (config_msg, exit_msg) = systray::launch_taskbar_icon();
    ui::main();
    loop {
        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
            println!("tray event: {event:?}");
        }

        if let Ok(event) = MenuEvent::receiver().try_recv() {
            println!("tray event: {event:?}");
            if let Ok(qid) = exit_msg.lock() {
                if let Some(menuid) = qid.as_ref() {
                    if menuid == event.id() {
                        println!("Got a quit");
                        break;
                    }
                }
            }
            if let Ok(cid) = config_msg.lock() {
                if let Some(menuid) = cid.as_ref() {
                    if menuid == event.id() {
                        println!("Launch configurator")
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_secs_f64(0.1));
    }
}
