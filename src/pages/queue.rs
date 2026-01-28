use ratatui::{
    crossterm::event::{KeyCode, KeyModifiers},
    prelude::*,
};

use crate::{
    app::Colors,
    events::{AppEvent, EventSender},
    jukebox::Jukebox,
    utils,
};

pub struct QueuePage {
    index: usize,
    scroll: usize,
    events: EventSender,
}

impl QueuePage {
    pub fn new(events: EventSender) -> Self {
        Self {
            index: 0,
            scroll: 0,
            events,
        }
    }

    pub fn on_enter(&self) {}

    pub fn on_render(&mut self, area: Rect, buf: &mut Buffer, jb: &Jukebox, colors: &Colors) {
        if jb.is_queue_empty() {
            utils::print_ascii(
                area,
                buf,
                "No tracks in the queue",
                Style::new().fg(colors.neutral),
                utils::Alignment::CenterHorizontal,
            );
            return;
        }

        self.scroll = utils::calculate_scroll(self.index, area.height, self.scroll);
        render_queue(area, buf, jb, self.scroll, self.index, colors);
    }

    pub fn on_input(&mut self, key: KeyCode, _modifiers: KeyModifiers, jb: &mut Jukebox) {
        match key {
            KeyCode::Down => {
                self.index = usize::min(self.index + 1, jb.queue_len().saturating_sub(1));
                self.events.send(AppEvent::Render);
            }
            KeyCode::Up => {
                self.index = self.index.saturating_sub(1);
                self.events.send(AppEvent::Render);
            }
            _ => {}
        }
    }

    pub fn on_exit(&self) {}
}

fn render_queue(
    area: Rect,
    buf: &mut Buffer,
    jb: &Jukebox,
    scroll: usize,
    index: usize,
    colors: &Colors,
) {
    let mut line_area = Rect { height: 1, ..area };

    jb.queue_iter()
        .enumerate()
        .skip(scroll)
        .take(area.height as usize)
        .for_each(|(i, (_id, track))| {
            let mut style = Style::new();
            if index == i {
                style.bg = Some(colors.accent);
                style.fg = Some(colors.on_accent);
            }

            utils::print_line_iter(
                line_area,
                buf,
                [track.title(), " ", track.artist(), " ", track.album()],
                style,
            );

            line_area.y += 1;
        });
}
