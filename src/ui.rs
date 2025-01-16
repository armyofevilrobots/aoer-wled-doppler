use std::collections::HashMap;

use iced::widget::{button, column, horizontal_space, row, vertical_space, Row};
use iced::{Center, Theme};
// use iced_table::{table, Column};

pub fn main() -> iced::Result {
    // iced::run("WLED-Doppler", ConfigAppState::update, ConfigAppState::view)
    iced::application("WLED-Doppler", ConfigAppState::update, ConfigAppState::view)
        .theme(|_| Theme::Dark)
        .centered()
        .run()
}

#[derive(Default)]
struct ConfigAppState {
    cfg_state: crate::types::Config,
    discovered_wleds: HashMap<String, u8, bool>, //DNSName, brightness, alive?
}

#[derive(Debug, Clone, Copy)]
enum Interaction {
    NA,
}

impl ConfigAppState {
    fn update(&mut self, message: Interaction) {
        match message {
            Interaction::NA => {}
        }
    }

    fn view(&self) -> Row<Interaction> {
        row![
            horizontal_space(),
            column![
                vertical_space(),
                // table(Id::new("WLEDs"), Id::new("BODY"), [Column]),
                button("FOO").on_press(Interaction::NA),
                
                vertical_space(),
            ]
            .padding(20)
            .width(500)
            .align_x(Center),
            horizontal_space(),
        ]
    }
}

mod test {
    
}
