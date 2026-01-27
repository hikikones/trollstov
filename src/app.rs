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
    widgets::{Shortcut, Shortcuts, TextSegment},
};

pub struct App {
    running: bool,
    pages: Pages,
    route: Route,
    colors: Colors,
    events: EventHandler,
    jukebox: Jukebox,
    current: Option<TrackId>,
    navigation: TextSegment,
    playing_title: String,
    playing_status_line: Line<'static>,
    playing_current_duration: String,
    playing_total_duration: String,
    shortcuts_app: Shortcuts<'static>,
    shortcuts_page: Shortcuts<'static>,
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
    pub fn new(
        events: EventHandler,
        jukebox: Jukebox,
        picker: ratatui_image::picker::Picker,
    ) -> Self {
        let colors = Colors::new();
        let pages = Pages::new(picker, events.clone_sender(), &colors);

        let mut shortcuts_app = Shortcuts::new(colors.neutral, colors.accent);
        shortcuts_app.extend([
            Shortcut::new("Quit", "Esc"),
            Shortcut::new("Navigate", "(⇧)Tab"),
            Shortcut::new("Play/Pause", "^￪"),
            Shortcut::new("Next/Prev", "^⇆"),
            Shortcut::new("Stop", "^￬"),
            Shortcut::new("Search", "/"),
            Shortcut::new("Seek 30s", "⎇→"),
        ]);
        let shortcuts_page = Shortcuts::new(Color::Reset, colors.accent);

        Self {
            running: true,
            pages,
            route: Route::default(),
            colors,
            events,
            jukebox,
            current: None,
            navigation: TextSegment::new(),
            playing_title: String::new(),
            playing_status_line: Line::default().centered(),
            playing_current_duration: String::with_capacity(5),
            playing_total_duration: String::from("00:00"),
            shortcuts_app,
            shortcuts_page,
        }
    }

    pub fn run(&mut self, mut terminal: Terminal) -> Result<(), Box<dyn std::error::Error>> {
        // Render initial page
        match self.route {
            Route::Tracks => self.pages.tracks.on_enter(),
            Route::NowPlaying => self.pages.now_playing.on_enter(),
            Route::Queue => self.pages.queue.on_enter(),
            Route::Search => self.pages.search.on_enter(),
            Route::Logs => self.pages.logs.on_enter(),
        }
        self.render(&mut terminal)?;

        // Start reading events and load music
        self.events.start();
        self.jukebox.load();

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
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
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
                if ctrl {
                    self.jukebox.pause_or_play();
                    None
                } else {
                    Some(key)
                }
            }
            KeyCode::Down => {
                if ctrl {
                    self.jukebox.stop();
                    None
                } else {
                    Some(key)
                }
            }
            KeyCode::Right => {
                if ctrl {
                    self.jukebox.play_next();
                    None
                } else if alt {
                    self.jukebox.seek(std::time::Duration::from_secs(30));
                    None
                } else {
                    Some(key)
                }
            }
            KeyCode::Left => {
                if ctrl {
                    self.jukebox.play_previous();
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
                    None
                }
                MediaKeyCode::Stop => todo!(),
                MediaKeyCode::TrackNext => {
                    self.jukebox.play_next();
                    None
                }
                MediaKeyCode::TrackPrevious => todo!(),
                _ => None,
            },
            KeyCode::Char(c) => match c {
                '/' => {
                    if self.route == Route::Search {
                        Some(key)
                    } else {
                        self.events.send(AppEvent::Route(Route::Search));
                        None
                    }
                }
                _ => Some(key),
            },
            _ => Some(key),
        };

        if let Some(key) = pass_on_key_event {
            match self.route {
                Route::Tracks => {
                    self.pages
                        .tracks
                        .on_input(key.code, key.modifiers, &mut self.jukebox)
                }
                Route::NowPlaying => self.pages.now_playing.on_input(key.code, key.modifiers),
                Route::Queue => {
                    self.pages
                        .queue
                        .on_input(key.code, key.modifiers, &mut self.jukebox)
                }
                Route::Search => {
                    self.pages
                        .search
                        .on_input(key.code, key.modifiers, &mut self.jukebox)
                }
                Route::Logs => self.pages.logs.on_input(key.code, key.modifiers),
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
                    Route::Queue => self.pages.queue.on_exit(),
                    Route::Search => self.pages.search.on_exit(),
                    Route::Logs => self.pages.logs.on_exit(),
                }

                self.route = route;

                match route {
                    Route::Tracks => self.pages.tracks.on_enter(),
                    Route::NowPlaying => self.pages.now_playing.on_enter(),
                    Route::Queue => self.pages.queue.on_enter(),
                    Route::Search => self.pages.search.on_enter(),
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
        self.jukebox.update();
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
                body_area,
                shortcuts_page_area,
                _,
                playing_title_area,
                playing_status_area,
                shortcuts_app_area,
            ] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Fill(0),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(area);

            // Title
            const TITLE: &str = "jukebox";
            buf.set_stringn(
                title_area.x + (title_area.width.saturating_sub(TITLE.len() as u16)) / 2,
                title_area.y,
                TITLE,
                TITLE.len(),
                Style::new().fg(self.colors.neutral),
            );

            // Navigation
            for (route, spacing) in [
                (Route::Tracks, "   "),
                (Route::NowPlaying, "   "),
                (Route::Queue, "   "),
                (Route::Search, "   "),
                (Route::Logs, ""),
            ] {
                let style = if route == self.route {
                    Style::new().bold().fg(self.colors.accent)
                } else {
                    Style::new()
                };

                self.navigation.push(route.title(), style);
                if route == Route::Logs {
                    let new_logs = self.pages.logs.queue_len();
                    if new_logs > 0 {
                        let mut buffer = itoa::Buffer::new();
                        self.navigation.extend([
                            ("(", style),
                            (buffer.format(new_logs), style),
                            (")", style),
                        ]);
                    }
                }
                self.navigation.push(spacing, Style::new());
            }
            self.navigation.render(
                utils::align(
                    Rect {
                        width: self.navigation.width(),
                        height: 1,
                        ..nav_area
                    },
                    nav_area,
                    utils::Alignment::CenterHorizontal,
                ),
                buf,
            );
            self.navigation.clear();

            // Body
            const MAX_WIDTH: u16 = 128;
            const MARGIN: u16 = 1;
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
                        &mut self.shortcuts_page,
                    );
                }
                Route::NowPlaying => {
                    self.pages
                        .now_playing
                        .on_render(body, buf, &self.jukebox, &self.colors);
                }
                Route::Queue => {
                    self.pages
                        .queue
                        .on_render(body, buf, &self.jukebox, &self.colors);
                }
                Route::Search => {
                    self.pages
                        .search
                        .on_render(body, buf, &self.jukebox, &self.colors);
                }
                Route::Logs => {
                    self.pages.logs.on_render(body, buf, &self.colors);
                }
            }
            self.shortcuts_page.render(shortcuts_page_area, buf);
            self.shortcuts_page.clear();

            // Playing
            if self.current != self.jukebox.current_track() {
                // Update currently playing
                self.current = self.jukebox.current_track();
                self.playing_title.clear();
                self.playing_total_duration.clear();

                match self.jukebox.current_track() {
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

            match self.jukebox.current_track() {
                Some(id) => {
                    let track = self.jukebox.get(id).unwrap();
                    let current_duration = self.jukebox.current_audio_position();
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

            // Shortcuts
            self.shortcuts_app.render(shortcuts_app_area, buf);
        })
    }
}
