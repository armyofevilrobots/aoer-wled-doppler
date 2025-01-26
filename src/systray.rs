use std::sync::{Arc, Mutex};
use tray_icon::menu::{
    AboutMetadata, CheckMenuItem, CheckMenuItemBuilder, Menu, MenuId, MenuItem, PredefinedMenuItem,
};
use tray_icon::TrayIconBuilder;

pub(crate) fn load_icon() -> (tray_icon::menu::Icon, tray_icon::Icon) {
    // let path = concat!(env!("CARGO_MANIFEST_DIR"), "/resources/aoer_logo_2018.png");
    // let icon = load_icon(std::path::Path::new(path));
    let (icon_rgba, icon_width, icon_height) = {
        let buffer = include_bytes!("../resources/aoer_logo_2018.png");
        let image = image::load_from_memory(buffer) // open(path)
            .expect("Failed to open icon bin")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };
    let tray_icon = tray_icon::Icon::from_rgba(icon_rgba.clone(), icon_width, icon_height)
        .expect("Failed to open icon");
    let about_icon = tray_icon::menu::Icon::from_rgba(icon_rgba.clone(), icon_width, icon_height)
        .expect("Failed to open icon");
    (about_icon, tray_icon)
}

pub(crate) fn launch_taskbar_icon() -> (
    Arc<Mutex<Option<MenuId>>>,
    Arc<Mutex<Option<MenuId>>>,
    Arc<Mutex<Option<MenuId>>>,
) {
    let config_msg_moved: Arc<Mutex<Option<MenuId>>> = Arc::new(Mutex::new(None));
    let config_msg = Arc::clone(&config_msg_moved);
    let exit_msg_moved: Arc<Mutex<Option<MenuId>>> = Arc::new(Mutex::new(None));
    let exit_msg = Arc::clone(&exit_msg_moved);
    let enabled_msg_moved: Arc<Mutex<Option<MenuId>>> = Arc::new(Mutex::new(None));
    let enabled_msg = Arc::clone(&enabled_msg_moved);

    #[cfg(target_os = "linux")]
    std::thread::spawn(move || {
        let (about_icon, icon) = load_icon();
        let tray_menu = Menu::new();
        let config_item: MenuItem = MenuItem::new("Web&UI", true, None);
        let quit_item: MenuItem = MenuItem::new("E&xit", true, None);
        let enabled_item = CheckMenuItemBuilder::new()
            .checked(true)
            .enabled(true)
            .text("Enabled")
            .build();

        tray_menu
            .append_items(&[
                &PredefinedMenuItem::about(
                    Some("About"),
                    Some(AboutMetadata {
                        name: Some("aoer-wled-doppler".to_string()),
                        copyright: Some("Copyright ArmyOfEvilRobots".to_string()),
                        version: option_env!("CARGO_PKG_VERSION")
                            .map(|version| version.to_string()),
                        icon: Some(about_icon),
                        ..Default::default()
                    }),
                ),
                &PredefinedMenuItem::separator(),
                &enabled_item,
                &PredefinedMenuItem::separator(),
                &config_item,
                &PredefinedMenuItem::separator(),
                &quit_item,
            ])
            .expect("Unexpected failure building tray menu...");
        if let Ok(mut locked_cfg_menuid) = config_msg_moved.lock() {
            *locked_cfg_menuid = Some(config_item.id().clone());
        }
        if let Ok(mut locked_quit_menuid) = exit_msg_moved.lock() {
            *locked_quit_menuid = Some(quit_item.id().clone());
        }
        if let Ok(mut locked_enabled_menuid) = enabled_msg_moved.lock() {
            *locked_enabled_menuid = Some(enabled_item.id().clone());
        }

        gtk::init().expect("Failed to initialize GTK for taskbar icon.");
        let _tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_icon(icon)
            .build()
            .unwrap();

        gtk::main();
    });
    (config_msg, exit_msg, enabled_msg)
}
