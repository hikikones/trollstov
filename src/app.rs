use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{CompletedFrame, layout::Flex, prelude::*};

use crate::{
    events::{AppEvent, Event},
    jukebox::Jukebox,
    pages::{Pages, Route},
    terminal::Terminal,
};

pub struct App {
    jb: Jukebox,
    events: crate::events::EventHandler,
    route: Route,
    pages: Pages,
    colors: Colors,
    title_line: Line<'static>,
    nav_line: Line<'static>,
    menu_line: Line<'static>,
}

pub struct Colors {
    pub accent: Color,
    pub on_accent: Color,
    pub neutral: Color,
}

impl App {
    pub fn new(jb: Jukebox) -> Self {
        let colors =
            match terminal_colorsaurus::theme_mode(terminal_colorsaurus::QueryOptions::default())
                .unwrap_or(terminal_colorsaurus::ThemeMode::Dark)
            {
                terminal_colorsaurus::ThemeMode::Dark => Colors {
                    accent: Color::Yellow,
                    on_accent: Color::Black,
                    neutral: Color::DarkGray,
                },
                terminal_colorsaurus::ThemeMode::Light => Colors {
                    accent: Color::LightBlue,
                    on_accent: Color::Black,
                    neutral: Color::DarkGray,
                },
            };

        let title_line = Line::styled("snowflake", Style::new().fg(colors.neutral)).centered();

        Self {
            jb,
            events: crate::events::EventHandler::new(),
            route: Route::Tracks,
            pages: Pages::new(),
            colors,
            title_line,
            nav_line: Line::default().centered(),
            menu_line: Line::default().centered(),
        }
    }

    pub fn run(mut self, mut terminal: Terminal) -> color_eyre::Result<()> {
        // Render initial page
        self.pages.tracks.on_enter(&self.jb);
        self.render(&mut terminal)?;

        // Run event loop
        loop {
            match self.events.next()? {
                Event::Terminal(event) => {
                    match event {
                        CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                            self.handle_key_press(key);
                        }
                        _ => {}
                    };
                }
                Event::App(event) => match event {
                    AppEvent::Render => {
                        self.render(&mut terminal)?;
                    }
                    AppEvent::Route(route) => {
                        match self.route {
                            Route::Tracks => self.pages.tracks.on_exit(),
                            Route::NowPlaying => self.pages.now_playing.on_exit(),
                        }

                        self.route = route;

                        match route {
                            Route::Tracks => self.pages.tracks.on_enter(&self.jb),
                            Route::NowPlaying => self.pages.now_playing.on_enter(&mut self.jb),
                        }

                        self.render(&mut terminal)?;
                    }
                    AppEvent::Quit => break,
                },
            }
        }

        Ok(())
    }

    fn handle_key_press(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.events.send(AppEvent::Quit),
            KeyCode::Tab => match self.route {
                Route::Tracks => self.events.send(AppEvent::Route(Route::NowPlaying)),
                Route::NowPlaying => self.events.send(AppEvent::Route(Route::Tracks)),
            },
            KeyCode::BackTab => match self.route {
                Route::Tracks => self.events.send(AppEvent::Route(Route::NowPlaying)),
                Route::NowPlaying => self.events.send(AppEvent::Route(Route::Tracks)),
            },
            _ => match self.route {
                Route::Tracks => {
                    self.pages
                        .tracks
                        .on_input(key.code, key.modifiers, &self.events, &mut self.jb)
                }
                Route::NowPlaying => {
                    self.pages
                        .now_playing
                        .on_input(key.code, key.modifiers, &self.events)
                }
            },
        }
    }

    fn render<'a>(&'a mut self, terminal: &'a mut Terminal) -> std::io::Result<CompletedFrame<'a>> {
        terminal.draw(|frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();

            let [title_area, _, nav_area, menu_area, body_area, _] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .areas(area);

            // Title
            (&self.title_line).render(title_area, buf);

            // Navigation
            for route in [Route::Tracks, Route::NowPlaying] {
                let (name, is_current) = match route {
                    Route::Tracks => ("Tracks", matches!(self.route, Route::Tracks)),
                    Route::NowPlaying => ("Now Playing", matches!(self.route, Route::NowPlaying)),
                };
                let style = if is_current {
                    Style::new().bold().fg(self.colors.accent)
                } else {
                    Style::new()
                };
                self.nav_line
                    .extend([Span::styled(name, style), Span::raw("   ")]);
            }
            self.nav_line.spans.pop();
            (&self.nav_line).render(nav_area, buf);
            self.nav_line.spans.clear();

            // Body
            const MAX_WIDTH: u16 = 128;
            const MARGIN: u16 = 2;
            let body = center_horizontal(body_area, Constraint::Length(MAX_WIDTH + MARGIN))
                .inner(Margin::new(MARGIN, MARGIN));
            match self.route {
                Route::Tracks => {
                    self.pages.tracks.on_render(
                        body,
                        buf,
                        &self.jb,
                        &self.colors,
                        &mut self.menu_line,
                    );
                }
                Route::NowPlaying => {
                    self.pages.now_playing.on_render(body, buf, &self.jb);
                }
            }

            // Menu
            (&self.menu_line).render(menu_area, buf);
            self.menu_line.spans.clear();
        })
    }
}

fn center_horizontal(area: Rect, constraint: Constraint) -> Rect {
    let [area] = Layout::horizontal([constraint])
        .flex(Flex::Center)
        .areas(area);
    area
}
