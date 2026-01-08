use std::time::{Duration, Instant};

use color_eyre::eyre::WrapErr;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    CompletedFrame, crossterm,
    layout::{Constraint, Flex, Layout, Margin, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, Cell, HighlightSpacing, Paragraph, Row, Table, TableState, Widget},
};

use crate::{audio::Database, terminal::Terminal};

const RENDER_FREQUENCY: f64 = 1.0;

pub enum Action {
    None,
    Render,
    Quit,
}

#[derive(Debug)]
pub struct App {
    db: Database,
    selected: usize,
    table_state: TableState,
}

impl App {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            selected: 0,
            table_state: TableState::new().with_selected(0),
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
                            self.selected =
                                usize::min(self.selected + 1, self.db.len().saturating_sub(1));
                            self.table_state.select(self.selected.into());
                            Action::Render
                        }
                        KeyCode::Up => {
                            self.selected = self.selected.saturating_sub(1);
                            self.table_state.select(self.selected.into());
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
                Press `Esc` to quit.\n\
                Browse tracks with arrows and play music with media keys.\n",
            )
            .centered()
            .render(desc_area, buf);

            // Body
            const MAX_WIDTH: u16 = 72;
            const MARGIN: u16 = 2;
            let body = center_horizontal(body_area, Constraint::Length(MAX_WIDTH + MARGIN))
                .inner(Margin::new(MARGIN, MARGIN));

            let widths = [
                Constraint::Length(20),
                Constraint::Length(16),
                Constraint::Length(20),
                Constraint::Length(6),
                Constraint::Length(6),
            ];
            let rows = self.db.iter().map(|track| {
                Row::new([
                    track.title(),
                    track.artist(),
                    track.album(),
                    "1:23",
                    "*****",
                ])
            });
            let table = Table::new(rows, widths)
                .header(Row::new([
                    Cell::from("Title"),
                    Cell::from("Artist"),
                    Cell::from("Album"),
                    Cell::from("Time"),
                    Cell::from("Rating"),
                ]))
                .row_highlight_style(Style::new().reversed());

            frame.render_stateful_widget(table, body, &mut self.table_state);
        })
    }
}

fn center_horizontal(area: Rect, constraint: Constraint) -> Rect {
    let [area] = Layout::horizontal([constraint])
        .flex(Flex::Center)
        .areas(area);
    area
}
