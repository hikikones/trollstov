use std::path::Path;

use ratatui::{
    CompletedFrame,
    crossterm::event::{
        Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MediaKeyCode,
    },
    prelude::*,
};
use unicode_width::UnicodeWidthStr;

use crate::{
    events::{AppEvent, Event, EventHandler},
    jukebox::{Jukebox, TrackId},
    pages::{Pages, Route},
    terminal::Terminal,
    utils,
};

pub struct App {
    running: bool,
    pages: Pages,
    route: Route,
    colors: Colors,
    events: EventHandler,
    jukebox: Jukebox,
    current: Option<TrackId>,
    title_line: Line<'static>,
    nav_line: Line<'static>,
    menu_line: Line<'static>,
    playing_title: String,
    playing_status_line: Line<'static>,
    playing_current_duration: String,
    playing_total_duration: String,
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
    pub fn new(music_dir: impl AsRef<Path>) -> Self {
        // Create picker after entering alternate screen, but before reading terminal events
        let picker = ratatui_image::picker::Picker::from_query_stdio().unwrap();

        let pages = Pages::new(picker);
        let colors = Colors::new();
        let events = EventHandler::new();
        let jukebox = Jukebox::new(music_dir).unwrap();
        let title_line = Line::styled("jukebox", Style::new().fg(colors.neutral)).centered();

        Self {
            running: true,
            pages,
            route: Route::default(),
            colors,
            events,
            jukebox,
            current: None,
            title_line,
            nav_line: Line::default().centered(),
            menu_line: Line::default().centered(),
            playing_title: String::new(),
            playing_status_line: Line::default().centered(),
            playing_current_duration: String::with_capacity(5),
            playing_total_duration: String::from("00:00"),
        }
    }

    pub fn run(&mut self, mut terminal: Terminal) -> Result<(), Box<dyn std::error::Error>> {
        // Render initial page
        match self.route {
            Route::Tracks => self.pages.tracks.on_enter(&self.jukebox),
            Route::NowPlaying => self.pages.now_playing.on_enter(&mut self.jukebox),
            Route::Logs => self.pages.logs.on_enter(),
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

    pub fn quit(self) {
        self.jukebox.shutdown();
    }

    fn handle_key_press(&mut self, key: KeyEvent) {
        let pass_on_key_event = match key.code {
            KeyCode::Esc => {
                self.events.send(AppEvent::Quit);
                None
            }
            KeyCode::Tab => {
                self.events.send(AppEvent::Route(self.route.next()));
                None
            }
            KeyCode::BackTab => {
                self.events.send(AppEvent::Route(self.route.prev()));
                None
            }
            KeyCode::Up => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.jukebox.pause_or_play();
                    self.events.send(AppEvent::UpdateAndRender);
                    None
                } else {
                    Some(key)
                }
            }
            KeyCode::Down => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.jukebox.stop();
                    self.events.send(AppEvent::UpdateAndRender);
                    None
                } else {
                    Some(key)
                }
            }
            KeyCode::Right => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    let _ = self.jukebox.play_random();
                    self.events.send(AppEvent::UpdateAndRender);
                    None
                } else {
                    Some(key)
                }
            }
            KeyCode::Left => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    self.jukebox.play_previous();
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
                    self.jukebox.pause_or_play();
                    self.events.send(AppEvent::UpdateAndRender);
                    None
                }
                MediaKeyCode::Stop => todo!(),
                MediaKeyCode::TrackNext => {
                    let _ = self.jukebox.play_random();
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
                Route::Tracks => self.pages.tracks.on_input(
                    key.code,
                    key.modifiers,
                    &self.events,
                    &mut self.jukebox,
                ),
                Route::NowPlaying => {
                    self.pages
                        .now_playing
                        .on_input(key.code, key.modifiers, &self.events)
                }
                Route::Logs => self
                    .pages
                    .logs
                    .on_input(key.code, key.modifiers, &self.events),
            }
        }
    }

    fn handle_app_event(
        &mut self,
        event: AppEvent,
        terminal: &mut Terminal,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
                    Route::Logs => self.pages.logs.on_exit(),
                }

                self.route = route;

                match route {
                    Route::Tracks => self.pages.tracks.on_enter(&self.jukebox),
                    Route::NowPlaying => self.pages.now_playing.on_enter(&mut self.jukebox),
                    Route::Logs => self.pages.logs.on_enter(),
                }

                self.render(terminal)?;
            }
            AppEvent::Log(log) => {
                self.pages.logs.enqueue(log);
            }
            AppEvent::Quit => {
                self.running = false;
            }
        }

        Ok(())
    }

    fn update(&mut self) {
        self.jukebox.update(&self.events);
        self.pages.now_playing.on_update(&self.jukebox);
    }

    fn render<'a>(&'a mut self, terminal: &'a mut Terminal) -> std::io::Result<CompletedFrame<'a>> {
        terminal.draw(|frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();

            let [
                title_area,
                _,
                nav_area,
                menu_area,
                body_area,
                playing_title_area,
                playing_status_area,
            ] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Fill(0),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(area);

            // Title
            (&self.title_line).render(title_area, buf);

            // Navigation
            for route in [Route::Tracks, Route::NowPlaying, Route::Logs] {
                let (name, is_current) = match route {
                    Route::Tracks => ("Tracks", matches!(self.route, Route::Tracks)),
                    Route::NowPlaying => ("Now Playing", matches!(self.route, Route::NowPlaying)),
                    Route::Logs => ("Logs", matches!(self.route, Route::Logs)),
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
                        &self.jukebox,
                        &self.colors,
                        &mut self.menu_line,
                    );
                }
                Route::NowPlaying => {
                    self.pages
                        .now_playing
                        .on_render(body, buf, &self.jukebox, &self.colors);
                }
                Route::Logs => {
                    self.pages.logs.on_render(body, buf, &self.colors);
                }
            }

            // Menu
            (&self.menu_line).render(menu_area, buf);
            self.menu_line.spans.clear();

            // Playing
            if self.current != self.jukebox.current() {
                // Update currently playing
                self.current = self.jukebox.current();
                self.playing_title.clear();
                self.playing_total_duration.clear();

                match self.jukebox.current() {
                    Some(id) => {
                        let track = self.jukebox.get(id).unwrap();
                        self.playing_title.push_str(track.artist());
                        if !(track.artist().is_empty() || track.title().is_empty()) {
                            self.playing_title.push_str(" - ");
                        }
                        self.playing_title.push_str(track.title());
                        self.playing_total_duration
                            .push_str(track.duration_display());
                    }
                    None => {
                        self.playing_total_duration.push_str("00:00");
                    }
                }
            }

            let [left_time_area, status_area, right_time_area] = Layout::horizontal([
                Constraint::Fill(0),
                Constraint::Percentage(30),
                Constraint::Fill(0),
            ])
            .areas(playing_status_area);

            self.playing_status_line.spans.clear();

            match self.jukebox.current() {
                Some(id) => {
                    let track = self.jukebox.get(id).unwrap();
                    let current_duration = self.jukebox.pos();
                    let total_duration = track.duration();

                    self.playing_current_duration.clear();
                    utils::format_duration(current_duration, &mut self.playing_current_duration);

                    let progress = current_duration.as_secs_f32() / total_duration.as_secs_f32();
                    let max_highlight_bound = (status_area.width as f32 * progress) as u16;
                    for i in 0..status_area.width {
                        let (c, style) = if i <= max_highlight_bound {
                            ("━", Style::new().fg(self.colors.accent))
                        } else {
                            ("─", Style::new().fg(self.colors.neutral))
                        };
                        self.playing_status_line.spans.push(Span::styled(c, style));
                    }
                }
                None => {
                    self.playing_current_duration.clear();
                    self.playing_current_duration.push_str("00:00");
                    for _ in 0..status_area.width {
                        self.playing_status_line
                            .spans
                            .push(Span::styled("─", Style::new().fg(self.colors.neutral)));
                    }
                }
            }

            Span::styled(&self.playing_title, Style::new().fg(self.colors.neutral)).render(
                utils::align(
                    Rect {
                        width: self.playing_title.width() as u16,
                        ..playing_title_area
                    },
                    playing_title_area,
                    utils::Alignment::CenterHorizontal,
                ),
                buf,
            );

            Span::styled(
                &self.playing_current_duration,
                Style::new().fg(self.colors.neutral),
            )
            .render(
                utils::align(
                    Rect {
                        width: 6,
                        ..left_time_area
                    },
                    left_time_area,
                    utils::Alignment::Right,
                ),
                buf,
            );
            (&self.playing_status_line).render(status_area, buf);
            Span::styled(
                &self.playing_total_duration,
                Style::new().fg(self.colors.neutral),
            )
            .render(
                Rect {
                    x: right_time_area.x + 1,
                    width: 5,
                    ..right_time_area
                },
                buf,
            );
        })
    }
}
