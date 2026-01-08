use std::time::{Duration, Instant};

use color_eyre::eyre::WrapErr;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    CompletedFrame, crossterm,
    layout::Alignment,
    style::{Color, Stylize},
    widgets::{Block, BorderType, Paragraph, Widget},
};

use crate::terminal::Terminal;

const RENDER_FREQUENCY: f64 = 1.0;

pub enum Action {
    None,
    Render,
    Quit,
}

#[derive(Debug)]
pub struct App {
    counter: u8,
}

impl App {
    pub fn new() -> Self {
        Self { counter: 0 }
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
                self.increment_counter();
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
                        KeyCode::Left => {
                            self.decrement_counter();
                            Action::Render
                        }
                        KeyCode::Right => {
                            self.increment_counter();
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

    fn increment_counter(&mut self) {
        self.counter = self.counter.saturating_add(1);
    }

    fn decrement_counter(&mut self) {
        self.counter = self.counter.saturating_sub(1);
    }

    fn render<'a>(&'a mut self, terminal: &'a mut Terminal) -> std::io::Result<CompletedFrame<'a>> {
        terminal.draw(|frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();

            let block = Block::bordered()
                .title("Amazing Counter App")
                .title_alignment(Alignment::Center)
                .border_type(BorderType::Rounded);

            let text = format!(
                "This is a TUI template.\n\
                Press `Esc` to stop running.\n\
                Press left and right to increment and decrement the counter respectively.\n\
                Counter: {}",
                self.counter
            );

            let paragraph = Paragraph::new(text)
                .block(block)
                .fg(Color::Cyan)
                .bg(Color::Black)
                .centered();

            paragraph.render(area, buf);
        })
    }
}
