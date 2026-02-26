use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};

use crate::{
    app::Action,
    colors::Colors,
    widgets::{Shortcuts, utils},
};

pub struct SettingsPage {}

impl SettingsPage {
    pub const fn new(colors: &Colors) -> Self {
        Self {}
    }

    pub fn on_enter(&self) {}

    pub fn on_render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        colors: &Colors,
        shortcuts: &mut Shortcuts,
    ) {
        utils::print_ascii(area, buf, "TODO", colors.neutral, utils::Alignment::Center);
    }

    pub fn on_input(&mut self, _key: KeyCode, _modifiers: KeyModifiers) -> Action {
        Action::None
    }

    pub fn on_exit(&self) {}
}
