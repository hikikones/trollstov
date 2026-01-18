use std::path::Path;

use ratatui::{
    CompletedFrame,
    crossterm::event::{
        Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MediaKeyCode,
    },
    prelude::*,
};

use crate::{
    events::{AppEvent, Event, EventHandler},
    jukebox::{Jukebox, TrackId},
    pages::{Pages, Route},
    terminal::Terminal,
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
    play_title_line: Line<'static>,
    play_status_line: Line<'static>,
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
            play_title_line: Line::default().centered(),
            play_status_line: Line::default().centered(),
        }
    }

    pub fn run(mut self, mut terminal: Terminal) -> color_eyre::Result<()> {
        // Render initial page
        match self.route {
            Route::Tracks => self.pages.tracks.on_enter(&self.jukebox),
            Route::NowPlaying => self.pages.now_playing.on_enter(&mut self.jukebox),
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
                    Route::Tracks => self.pages.tracks.on_enter(&self.jukebox),
                    Route::NowPlaying => self.pages.now_playing.on_enter(&mut self.jukebox),
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
            }

            // Menu
            (&self.menu_line).render(menu_area, buf);
            self.menu_line.spans.clear();

            // Playing
            if self.current != self.jukebox.current() {
                // Update currently playing
                self.current = self.jukebox.current();
                self.play_title_line.spans.clear();
                // self.play_status_line.spans.clear();

                if let Some(id) = self.jukebox.current() {
                    let track = self.jukebox.get(id).unwrap();
                    let style = Style::new().fg(self.colors.neutral);
                    self.play_title_line.spans.extend([
                        Span::styled(track.artist().to_string(), style),
                        Span::styled(" - ", style),
                        Span::styled(track.title().to_string(), style),
                    ]);
                }

                // match self.jukebox.current() {
                //     Some(id) => {}
                //     None => {
                //         //todo
                //     }
                // }
            }

            match self.jukebox.current() {
                Some(id) => {
                    //todo
                    self.play_status_line.spans.clear();
                    let track = self.jukebox.get(id).unwrap();
                    let curr_dur = self.jukebox.pos();
                    let curr_dur_display = {
                        let seconds = curr_dur.as_secs() % 60;
                        format!("{:02}:{:02}", (curr_dur.as_secs() - seconds) / 60, seconds)
                    };

                    let total_dur = track.duration();
                    let total_dur_display = track.duration_display().to_string();
                    let perc = curr_dur.as_secs_f32() / total_dur.as_secs_f32();
                    let style = Style::new();
                    let dur = "00:00";
                    self.play_status_line.spans.extend([
                        Span::styled(curr_dur_display, style),
                        Span::styled(" ", style),
                    ]);
                    let a = playing_status_area.width / 3;
                    let perc_a = (a as f32 * perc) as u16;
                    for i in 0..a {
                        let style = if i < perc_a {
                            Style::new().blue()
                        } else if i == perc_a {
                            Style::new().red()
                        } else {
                            Style::new()
                        };
                        self.play_status_line.spans.push(Span::styled("-", style));
                    }
                    self.play_status_line.spans.extend([
                        Span::styled(" ", style),
                        Span::styled(total_dur_display, style),
                    ]);
                }
                None => {
                    //todo
                    self.play_status_line.spans.clear();
                    let style = Style::new();
                    let dur = "00:00";
                    self.play_status_line
                        .spans
                        .extend([Span::styled(dur, style), Span::styled(" ", style)]);
                    let a = playing_status_area.width / 3;
                    for _ in 0..a {
                        self.play_status_line.spans.push(Span::styled("-", style));
                    }
                    self.play_status_line
                        .spans
                        .extend([Span::styled(" ", style), Span::styled(dur, style)]);
                }
            }

            (&self.play_title_line).render(playing_title_area, buf);
            (&self.play_status_line).render(playing_status_area, buf);
            // self.play_title_line.spans.clear();
            // self.play_status_line.spans.clear();
        })
    }
}
