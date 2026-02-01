use std::time::Duration;

use ratatui::{
    CompletedFrame,
    crossterm::event::{
        Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MediaKeyCode,
    },
    prelude::*,
};

use crate::{
    events::{AppEvent, Event, EventHandler},
    jukebox::{Jukebox, Track},
    pages::{Pages, Route},
    terminal::Terminal,
    utils,
    widgets::{Shortcut, Shortcuts, TextSegment},
};

// TODO: Add scrolling bars to the various pages.
// TODO: Add a playlist page for artists/albums/genres and filtering.
// TODO: Handle most unwraps.

pub struct App {
    running: bool,
    pages: Pages,
    route: Route,
    colors: Colors,
    events: EventHandler,
    jukebox: Jukebox,
    text_segment: TextSegment,
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
            text_segment: TextSegment::new().with_alignment(Alignment::Center),
            shortcuts_app,
            shortcuts_page,
        }
    }

    pub fn run(&mut self, mut terminal: Terminal) -> Result<(), Box<dyn std::error::Error>> {
        // Render initial page
        // TODO: Show ASCII logo instead. Wait until jukebox.len() > 0.
        self.on_enter();
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
        match key.code {
            KeyCode::Esc => {
                self.events.send(AppEvent::Quit);
            }
            KeyCode::Tab => {
                self.events.send(AppEvent::Route(self.route.next()));
            }
            KeyCode::BackTab => {
                self.events.send(AppEvent::Route(self.route.prev()));
            }
            KeyCode::Up => {
                if ctrl {
                    self.jukebox.pause_or_play();
                } else {
                    self.on_input(key);
                }
            }
            KeyCode::Down => {
                if ctrl {
                    self.jukebox.stop();
                } else {
                    self.on_input(key);
                }
            }
            KeyCode::Right => {
                if ctrl {
                    self.jukebox.play_next();
                } else if alt {
                    self.jukebox.fast_forwards_by(Duration::from_secs(30));
                } else {
                    self.on_input(key);
                }
            }
            KeyCode::Left => {
                if ctrl {
                    self.jukebox.play_previous();
                } else {
                    self.on_input(key);
                }
            }
            KeyCode::Media(media) => match media {
                MediaKeyCode::Play => todo!(),
                MediaKeyCode::Pause => todo!(),
                MediaKeyCode::PlayPause => {
                    self.jukebox.pause_or_play();
                }
                MediaKeyCode::Stop => todo!(),
                MediaKeyCode::TrackNext => {
                    self.jukebox.play_next();
                }
                MediaKeyCode::TrackPrevious => todo!(),
                MediaKeyCode::Reverse => todo!(),
                MediaKeyCode::FastForward => todo!(),
                MediaKeyCode::Rewind => todo!(),
                MediaKeyCode::Record => todo!(),
                MediaKeyCode::LowerVolume => todo!(),
                MediaKeyCode::RaiseVolume => todo!(),
                MediaKeyCode::MuteVolume => todo!(),
            },
            KeyCode::Char(c) => match c {
                '/' => {
                    if self.route == Route::Search {
                        self.on_input(key);
                    } else {
                        self.events.send(AppEvent::Route(Route::Search));
                    }
                }
                _ => {
                    self.on_input(key);
                }
            },
            _ => {
                self.on_input(key);
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
                self.on_exit();
                self.route = route;
                self.on_enter();
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
        self.pages.playing.on_update(&self.jukebox);
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
                playback_title_area,
                playback_status_area,
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
            utils::print_ascii(
                title_area,
                buf,
                "jukebox",
                Style::new().fg(self.colors.neutral),
                utils::Alignment::CenterHorizontal,
            );

            // Navigation
            render_navigation(
                nav_area,
                buf,
                &mut self.text_segment,
                self.route,
                &self.pages,
                &self.colors,
            );

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
                        .playing
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

            // Playback
            match self.jukebox.current_track_id() {
                Some(id) => {
                    let track = self.jukebox.get(id).unwrap();

                    // Render playback title
                    render_playback_title(
                        playback_title_area,
                        buf,
                        &mut self.text_segment,
                        track,
                        &self.colors,
                    );

                    // Render active playback status
                    render_playback_status_active(
                        playback_status_area,
                        buf,
                        &mut self.text_segment,
                        self.jukebox.current_track_pos(),
                        track.duration(),
                        &self.colors,
                    );
                }
                None => {
                    // Render empty playback status
                    render_playback_status_empty(
                        playback_status_area,
                        buf,
                        &mut self.text_segment,
                        &self.colors,
                    );
                }
            }

            // Shortcuts
            self.shortcuts_app.render(shortcuts_app_area, buf);
        })
    }

    fn on_enter(&mut self) {
        match self.route {
            Route::Tracks => self.pages.tracks.on_enter(),
            Route::NowPlaying => self.pages.playing.on_enter(),
            Route::Search => self.pages.search.on_enter(),
            Route::Logs => self.pages.logs.on_enter(),
        }
    }

    fn on_exit(&mut self) {
        match self.route {
            Route::Tracks => self.pages.tracks.on_exit(),
            Route::NowPlaying => self.pages.playing.on_exit(),
            Route::Search => self.pages.search.on_exit(),
            Route::Logs => self.pages.logs.on_exit(),
        }
    }

    fn on_input(&mut self, key: KeyEvent) {
        match self.route {
            Route::Tracks => self
                .pages
                .tracks
                .on_input(key.code, key.modifiers, &mut self.jukebox),
            Route::NowPlaying => self.pages.playing.on_input(key.code, key.modifiers),
            Route::Search => self
                .pages
                .search
                .on_input(key.code, key.modifiers, &mut self.jukebox),
            Route::Logs => self.pages.logs.on_input(key.code, key.modifiers),
        }
    }
}

fn render_navigation(
    line: Rect,
    buf: &mut Buffer,
    text: &mut TextSegment,
    current_route: Route,
    pages: &Pages,
    colors: &Colors,
) {
    const SPACING: &str = "   ";
    for (route, spacing) in [
        (Route::Tracks, SPACING),
        (Route::NowPlaying, SPACING),
        (Route::Search, SPACING),
        (Route::Logs, ""),
    ] {
        let style = if route == current_route {
            Style::new().bold().fg(colors.accent)
        } else {
            Style::new()
        };

        text.push_str(route.title(), style);
        if route == Route::Logs {
            let new_logs = pages.logs.queue_len();
            if new_logs > 0 {
                let mut buffer = itoa::Buffer::new();
                text.extend([("(", style), (buffer.format(new_logs), style), (")", style)]);
            }
        }
        text.push_str(spacing, Style::new());
    }

    text.render(line, buf);
    text.clear();
}

fn render_playback_title(
    line: Rect,
    buf: &mut Buffer,
    text: &mut TextSegment,
    track: &Track,
    colors: &Colors,
) {
    let neutral_style = Style::new().fg(colors.neutral);

    text.push_str(track.artist(), neutral_style);
    if !(track.artist().is_empty() || track.title().is_empty()) {
        text.push_str(" - ", neutral_style);
    }
    text.push_str(track.title(), neutral_style);

    text.render(line, buf);
    text.clear();
}

fn render_playback_status_active(
    line: Rect,
    buf: &mut Buffer,
    text: &mut TextSegment,
    current_duration: Duration,
    total_duration: Duration,
    colors: &Colors,
) {
    let accent_style = Style::new().fg(colors.accent);
    let neutral_style = Style::new().fg(colors.neutral);

    text.push_chars(
        &utils::format_duration_on_stack(current_duration),
        neutral_style,
    );
    text.push_char(' ', neutral_style);

    let status_width = line.width / 3;
    let progress = current_duration.as_secs_f32() / total_duration.as_secs_f32();
    let max_highlight_bound = (status_width as f32 * progress) as u16;
    for i in 0..status_width {
        let (ch, style) = if i <= max_highlight_bound {
            ('━', accent_style)
        } else {
            ('─', neutral_style)
        };
        text.push_char(ch, style);
    }

    text.push_char(' ', neutral_style);
    text.push_chars(
        &utils::format_duration_on_stack(total_duration),
        neutral_style,
    );

    text.render(line, buf);
    text.clear();
}

fn render_playback_status_empty(
    line: Rect,
    buf: &mut Buffer,
    text: &mut TextSegment,
    colors: &Colors,
) {
    let style = Style::new().fg(colors.neutral);
    text.push_str("00:00 ", style);
    let status_width = line.width / 3;
    for _ in 0..status_width {
        text.push_char('─', style);
    }
    text.push_str(" 00:00", style);

    text.render(line, buf);
    text.clear();
}
