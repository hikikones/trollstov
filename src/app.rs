use std::time::{Duration, Instant};

use color_eyre::eyre::WrapErr;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    CompletedFrame, crossterm,
    layout::{Constraint, Flex, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::{
    audio::{AudioPlayback, Database},
    terminal::Terminal,
};

const RENDER_FREQUENCY: f64 = 1.0;

pub enum Action {
    None,
    Render,
    Quit,
}

pub struct App {
    db: Database,
    audio: AudioPlayback,
    current: usize,
    scroll: usize,
    selected: Option<usize>,
}

impl App {
    pub fn new(db: Database, audio: AudioPlayback) -> Self {
        Self {
            db,
            audio,
            current: 0,
            scroll: 0,
            selected: None,
        }
    }

    pub fn run(mut self, mut terminal: Terminal) -> color_eyre::Result<()> {
        self.render(&mut terminal)?;

        // Setup render timers
        let render_interval = Duration::from_secs_f64(1.0 / RENDER_FREQUENCY);
        let mut last_render = Instant::now();

        loop {
            // Render at a fixed rate
            let render_timeout = render_interval.saturating_sub(last_render.elapsed());
            if render_timeout == Duration::ZERO {
                last_render = Instant::now();
                self.render(&mut terminal)?;
            }

            // Poll for crossterm event in a non-blocking manner
            if crossterm::event::poll(render_timeout)
                .wrap_err("failed to poll for crossterm events")?
            {
                let event = crossterm::event::read().wrap_err("failed to read crossterm event")?;

                // Retrieve action from event
                let action = match event {
                    Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Esc => Action::Quit,
                        KeyCode::Down => {
                            self.current =
                                usize::min(self.current + 1, self.db.len().saturating_sub(1));
                            Action::Render
                        }
                        KeyCode::Up => {
                            self.current = self.current.saturating_sub(1);
                            Action::Render
                        }
                        KeyCode::Enter => {
                            self.selected = Some(self.current);
                            let track = self.db.iter().nth(self.current).unwrap();
                            let _ = self.audio.play(track.path());
                            Action::Render
                        }
                        _ => Action::None,
                    },
                    Event::Resize(_, _) => Action::Render,
                    _ => Action::None,
                };

                // Apply action
                match action {
                    Action::None => {}
                    Action::Render => {
                        self.render(&mut terminal)?;
                    }
                    Action::Quit => break,
                }
            }
        }

        Ok(())
    }

    fn render<'a>(&'a mut self, terminal: &'a mut Terminal) -> std::io::Result<CompletedFrame<'a>> {
        terminal.draw(|frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();

            let [title_area, _, desc_area, body_area] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(0),
            ])
            .areas(area);

            // Title
            Line::from("Snowflake").centered().render(title_area, buf);

            // Description
            Paragraph::new(
                "A simple music app for the terminal.\n\
                Press Esc to quit.\n\
                Browse music with arrow keys and play with Enter.",
            )
            .centered()
            .render(desc_area, buf);

            // Table of tracks
            const MAX_WIDTH: u16 = 128;
            const MARGIN: u16 = 2;
            let body = center_horizontal(body_area, Constraint::Length(MAX_WIDTH + MARGIN))
                .inner(Margin::new(MARGIN, MARGIN));

            let spacing = 2;
            let time_width = 6 + spacing;
            let rating_width = 6;
            let remaining_width = body.width.saturating_sub(time_width + rating_width);
            let info_width = remaining_width / 3;

            let [header_area, table_area] =
                Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(body);

            let mut x = header_area.x;
            for (header, width, spacing) in [
                ("Title", info_width, spacing),
                ("Artist", info_width, spacing),
                ("Album", info_width, spacing),
                ("Time", time_width, spacing),
                ("Rating", rating_width, 0),
            ] {
                let col = Rect {
                    width: width.saturating_sub(spacing),
                    height: 1,
                    x,
                    y: header_area.y,
                };
                Span::raw(header).render(col, buf);
                x += width;
            }

            let height = table_area.height as usize;
            if self.current > self.scroll {
                let height_diff = self.current - self.scroll;
                let height = height.saturating_sub(1);
                if height_diff > height {
                    self.scroll += height_diff - height;
                }
            } else if self.scroll > self.current {
                let height_diff = self.scroll - self.current;
                self.scroll -= height_diff;
            }

            let mut x = table_area.x;
            let mut y = table_area.y;

            self.db
                .iter()
                .enumerate()
                .skip(self.scroll)
                .take(height)
                .for_each(|(i, track)| {
                    for (text, width, spacing) in [
                        (track.title(), info_width, spacing),
                        (track.artist(), info_width, spacing),
                        (track.album(), info_width, spacing),
                        (track.duration_display(), time_width, spacing),
                        (track.rating_display(), rating_width, 0),
                    ] {
                        let col = Rect {
                            width: width.saturating_sub(spacing),
                            height: 1,
                            x,
                            y,
                        };

                        let mut style = Style::new();
                        if self.current == i {
                            style.fg = Some(Color::Yellow);
                        }
                        if let Some(selected) = self.selected
                            && selected == i
                        {
                            style.add_modifier.insert(Modifier::BOLD);
                        }

                        Span::styled(text, style).render(col, buf);
                        x += width;
                    }
                    x = table_area.x;
                    y += 1;
                });
        })
    }
}

fn center_horizontal(area: Rect, constraint: Constraint) -> Rect {
    let [area] = Layout::horizontal([constraint])
        .flex(Flex::Center)
        .areas(area);
    area
}
