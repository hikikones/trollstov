use crossterm::event::{
    Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MediaKeyCode,
};
use ratatui::{CompletedFrame, prelude::*};

use crate::{
    events::{AppEvent, Event, EventHandler},
    jukebox::Jukebox,
    pages::{Pages, Route},
    terminal::Terminal,
};

pub struct App {
    jb: Jukebox,
    running: bool,
    events: EventHandler,
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

impl Colors {
    pub fn new() -> Self {
        match terminal_colorsaurus::theme_mode(terminal_colorsaurus::QueryOptions::default())
            .unwrap_or(terminal_colorsaurus::ThemeMode::Dark)
        {
            terminal_colorsaurus::ThemeMode::Dark => Self {
                accent: Color::Yellow,
                on_accent: Color::Black,
                neutral: Color::DarkGray,
            },
            terminal_colorsaurus::ThemeMode::Light => Self {
                accent: Color::LightBlue,
                on_accent: Color::Black,
                neutral: Color::DarkGray,
            },
        }
    }
}

impl App {
    pub fn new(jb: Jukebox) -> Self {
        let colors = Colors::new();
        let title_line = Line::styled("jukebox", Style::new().fg(colors.neutral)).centered();

        Self {
            jb,
            running: true,
            events: EventHandler::new(),
            route: Route::default(),
            pages: Pages::new(),
            colors,
            title_line,
            nav_line: Line::default().centered(),
            menu_line: Line::default().centered(),
        }
    }

    pub fn run(mut self, mut terminal: Terminal) -> color_eyre::Result<()> {
        // Render initial page
        match self.route {
            Route::Tracks => self.pages.tracks.on_enter(&self.jb),
            Route::NowPlaying => self.pages.now_playing.on_enter(&mut self.jb),
        }
        self.render(&mut terminal)?;

        // Run event loop
        while self.running {
            match self.events.next()? {
                Event::Terminal(event) => match event {
                    CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                        self.handle_key_press(key);
                    }
                    _ => {}
                },
                Event::App(event) => {
                    self.handle_app_event(event, &mut terminal)?;
                }
            }
        }

        Ok(())
    }

    fn handle_key_press(&mut self, key: KeyEvent) {
        let pass_on_key_event = match key.code {
            KeyCode::Esc => {
                self.events.send(AppEvent::Quit);
                None
            }
            KeyCode::Tab => match self.route {
                Route::Tracks => {
                    self.events.send(AppEvent::Route(Route::NowPlaying));
                    None
                }
                Route::NowPlaying => {
                    self.events.send(AppEvent::Route(Route::Tracks));
                    None
                }
            },
            KeyCode::BackTab => match self.route {
                Route::Tracks => {
                    self.events.send(AppEvent::Route(Route::NowPlaying));
                    None
                }
                Route::NowPlaying => {
                    self.events.send(AppEvent::Route(Route::Tracks));
                    None
                }
            },
            KeyCode::Up => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.jb.pause_or_play();
                    self.events.send(AppEvent::UpdateAndRender);
                    None
                } else {
                    Some(key)
                }
            }
            KeyCode::Down => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.jb.stop();
                    self.events.send(AppEvent::UpdateAndRender);
                    None
                } else {
                    Some(key)
                }
            }
            KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    let _ = self.jb.play_random();
                    self.events.send(AppEvent::UpdateAndRender);
                    None
                } else {
                    Some(key)
                }
            }
            KeyCode::Left => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.jb.play_previous();
                    self.events.send(AppEvent::UpdateAndRender);
                    None
                } else {
                    Some(key)
                }
            }
            KeyCode::Media(media) => match media {
                MediaKeyCode::Play => todo!(),
                MediaKeyCode::Pause => todo!(),
                MediaKeyCode::PlayPause => {
                    self.jb.pause_or_play();
                    self.events.send(AppEvent::UpdateAndRender);
                    None
                }
                MediaKeyCode::Stop => todo!(),
                MediaKeyCode::TrackNext => {
                    let _ = self.jb.play_random();
                    self.events.send(AppEvent::UpdateAndRender);
                    None
                }
                MediaKeyCode::TrackPrevious => todo!(),
                _ => None,
            },
            _ => Some(key),
        };

        if let Some(key) = pass_on_key_event {
            match self.route {
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
            }
        }
    }

    fn handle_app_event(
        &mut self,
        event: AppEvent,
        terminal: &mut Terminal,
    ) -> color_eyre::Result<()> {
        match event {
            AppEvent::Update => {
                self.update();
            }
            AppEvent::Render => {
                self.render(terminal)?;
            }
            AppEvent::UpdateAndRender => {
                self.update();
                self.render(terminal)?;
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

                self.render(terminal)?;
            }
            AppEvent::Quit => {
                self.running = false;
            }
        }

        Ok(())
    }

    fn update(&mut self) {
        self.jb.update();
        self.pages.now_playing.on_update(&self.jb);
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
            let body = body_area
                .centered_horizontally(Constraint::Length(MAX_WIDTH + MARGIN))
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
                    self.pages
                        .now_playing
                        .on_render(body, buf, &self.jb, &self.colors);
                }
            }

            // Menu
            (&self.menu_line).render(menu_area, buf);
            self.menu_line.spans.clear();
        })
    }
}
