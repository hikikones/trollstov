use std::time::Duration;

use image::GenericImageView;
use jukebox::{AudioFileReport, AudioPicture, Jukebox, JukeboxEvent, Track};
use ratatui::{
    CompletedFrame,
    crossterm::event::{
        Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MediaKeyCode,
    },
    prelude::*,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};

use crate::{
    events::{AppEvent, Event, EventHandler},
    pages::{Log, Pages, Route},
    terminal::Terminal,
    widgets::{Shortcut, Shortcuts, TextSegment, utils},
};

// TODO: Add a playlist page for artists/albums/genres and filtering.

type FrontCoverHandle = std::thread::JoinHandle<Result<FrontCover, AudioFileReport>>;

pub struct App {
    running: bool,
    pages: Pages,
    route: Route,
    colors: Colors,
    events: EventHandler,
    jukebox: Jukebox,
    picker: Picker,
    front_cover: FrontCover,
    front_cover_handle: Option<FrontCoverHandle>,
    text_segment: TextSegment,
    shortcuts_page: Shortcuts,
    shortcuts_play: Shortcuts,
    shortcuts_app: Shortcuts,
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

pub enum FrontCover {
    None,
    Loading,
    Ready(StatefulProtocol),
}

impl App {
    pub fn new(events: EventHandler, jukebox: Jukebox, picker: Picker) -> Self {
        let colors = Colors::new();
        let pages = Pages::new(events.clone_sender(), &colors);

        let shortcuts_page = Shortcuts::new(Color::Reset, colors.accent);
        let mut shortcuts_play = Shortcuts::new(colors.neutral, colors.accent);
        shortcuts_play.extend([
            Shortcut::new("Play/Pause", "^￪"),
            Shortcut::new("Next/Prev", "^⇆"),
            Shortcut::new("Stop", "^￬"),
            Shortcut::new("Forward 30s", "⎇→"),
        ]);
        let mut shortcuts_app = Shortcuts::new(colors.neutral, colors.accent);
        shortcuts_app.extend([
            Shortcut::new("Quit", "Esc"),
            Shortcut::new("Navigate", "(⇧)Tab"),
            Shortcut::new("Search", "/"),
        ]);

        Self {
            running: true,
            pages,
            route: Route::default(),
            colors,
            events,
            jukebox,
            picker,
            front_cover: FrontCover::None,
            front_cover_handle: None,
            text_segment: TextSegment::new().with_alignment(Alignment::Center),
            shortcuts_page,
            shortcuts_play,
            shortcuts_app,
        }
    }

    pub fn run(&mut self, mut terminal: Terminal) -> Result<(), Box<dyn std::error::Error>> {
        // Draw logo
        terminal.draw(|frame| {
            render_jukebox_logo(frame.area(), frame.buffer_mut());
        })?;

        // Start reading events and load music
        self.events.start();
        self.jukebox.load_music();

        // Try to establish media controls
        if let Err(err) = self.jukebox.attach_media_controls() {
            let log = Log::new(err);
            self.events.send(AppEvent::Log(log));
        }

        self.on_enter();

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
                } else if alt {
                    let new_volume = (self.jukebox.volume() + 0.1).min(2.0);
                    self.jukebox.set_volume(new_volume);
                    self.events.send(AppEvent::Render);
                } else {
                    self.on_input(key);
                }
            }
            KeyCode::Down => {
                if ctrl {
                    self.jukebox.stop();
                } else if alt {
                    let new_volume = (self.jukebox.volume() - 0.1).max(0.0);
                    self.jukebox.set_volume(new_volume);
                    self.events.send(AppEvent::Render);
                } else {
                    self.on_input(key);
                }
            }
            KeyCode::Right => {
                if ctrl {
                    self.jukebox.play_next();
                } else if alt {
                    self.jukebox.fast_forward_by(Duration::from_secs(30));
                    self.events.send(AppEvent::Render);
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
                MediaKeyCode::Play => {
                    self.jukebox.play();
                }
                MediaKeyCode::Pause => {
                    self.jukebox.pause();
                }
                MediaKeyCode::PlayPause => {
                    self.jukebox.pause_or_play();
                }
                MediaKeyCode::Stop => {
                    self.jukebox.stop();
                }
                MediaKeyCode::TrackNext => {
                    self.jukebox.play_next();
                }
                MediaKeyCode::TrackPrevious => {
                    self.jukebox.play_previous();
                }
                MediaKeyCode::FastForward => {
                    self.jukebox.fast_forward_by(Duration::from_secs(30));
                }
                _ => {}
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
        let mut render = false;

        self.jukebox.update(|event| match event {
            JukeboxEvent::Play(_, path) => {
                // Load image in thread and store handle
                self.front_cover = FrontCover::Loading;
                let picker = self.picker.clone();
                let handle = std::thread::spawn(move || {
                    let picture = AudioPicture::read(&path)?;
                    match picture.bytes() {
                        Some(bytes) => {
                            const MAX_RES: u32 = 720;
                            let mut dyn_img = image::load_from_memory(bytes).map_err(|err| {
                                AudioFileReport::new(format!(
                                    "Could not load front cover image for {} due to {}",
                                    path.display(),
                                    err
                                ))
                            })?;
                            let (w, h) = dyn_img.dimensions();
                            if w > MAX_RES || h > MAX_RES {
                                dyn_img = dyn_img.thumbnail(MAX_RES, MAX_RES);
                            }
                            Ok(FrontCover::Ready(picker.new_resize_protocol(dyn_img)))
                        }
                        None => Ok(FrontCover::None),
                    }
                });
                self.front_cover_handle = Some(handle);
                render = true;
            }
            JukeboxEvent::Stop => {
                render = true;
            }
            JukeboxEvent::Rating(_) => {
                render = true;
            }
            JukeboxEvent::Error(err) => {
                render = true;
                let log = Log::new(err);
                self.events.send(AppEvent::Log(log));
            }
            JukeboxEvent::Focus => {
                // TODO: Focus terminal window.
            }
            JukeboxEvent::Quit => {
                self.running = false;
            }
        });

        // Poll thread for finished image loading
        if let Some(handle) = self.front_cover_handle.as_ref() {
            if handle.is_finished() {
                render = true;
                let handle = self.front_cover_handle.take().unwrap();
                match handle.join().unwrap() {
                    Ok(image) => {
                        self.front_cover = image;
                    }
                    Err(err) => {
                        let log = Log::new(err);
                        self.events.send(AppEvent::Log(log));
                        self.front_cover = FrontCover::None;
                    }
                }
            }
        }

        if render {
            self.events.send(AppEvent::Render);
        }
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
                shortcuts_play_area,
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
                    self.pages.playing.on_render(
                        body,
                        buf,
                        &self.jukebox,
                        &mut self.front_cover,
                        &self.colors,
                        &mut self.shortcuts_page,
                    );
                }
                Route::Search => {
                    self.pages
                        .search
                        .on_render(body, buf, &mut self.jukebox, &self.colors);
                }
                Route::Logs => {
                    self.pages.logs.on_render(body, buf, &self.colors);
                }
            }
            self.shortcuts_page.render(shortcuts_page_area, buf);
            self.shortcuts_page.clear();

            // Playback
            match self
                .jukebox
                .current_track_id()
                .and_then(|id| self.jukebox.get(id))
            {
                Some(track) => {
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
            let volume = (self.jukebox.volume() * 100.0).round() as u8;
            jukebox::utils::format_int(volume, |volume| {
                self.shortcuts_play
                    .push_iter(["Volume ", volume, "%"], "⎇⇵");
            });
            self.shortcuts_play.render(shortcuts_play_area, buf);
            self.shortcuts_app.render(shortcuts_app_area, buf);

            self.shortcuts_play.pop();
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
            Route::NowPlaying => {
                self.pages
                    .playing
                    .on_input(key.code, key.modifiers, &mut self.jukebox)
            }
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
    for (route, name, spacing) in [
        (Route::Tracks, "Tracks", SPACING),
        (Route::NowPlaying, "Now Playing", SPACING),
        (Route::Search, "Search", SPACING),
        (Route::Logs, "Logs", ""),
    ] {
        let style = if route == current_route {
            Style::new().bold().fg(colors.accent)
        } else {
            Style::new()
        };

        text.push_str(name, style);
        if route == Route::Logs {
            let new_logs = pages.logs.queue_len();
            if new_logs > 0 {
                jukebox::utils::format_int(new_logs, |new_logs| {
                    text.extend([("(", style), (new_logs, style), (")", style)]);
                });
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
        &jukebox::utils::format_duration_on_stack(current_duration),
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
        &jukebox::utils::format_duration_on_stack(total_duration),
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

fn render_jukebox_logo(area: Rect, buf: &mut Buffer) {
    const LOGO_TEXT: &str = r#"
                     /$$                 /$$                          
                    | $$                | $$                          
       /$$ /$$   /$$| $$   /$$  /$$$$$$ | $$$$$$$   /$$$$$$  /$$   /$$
      |__/| $$  | $$| $$  /$$/ /$$__  $$| $$__  $$ /$$__  $$|  $$ /$$/
       /$$| $$  | $$| $$$$$$/ | $$$$$$$$| $$  \ $$| $$  \ $$ \  $$$$/ 
      | $$| $$  | $$| $$_  $$ | $$_____/| $$  | $$| $$  | $$  >$$  $$ 
      | $$|  $$$$$$/| $$ \  $$|  $$$$$$$| $$$$$$$/|  $$$$$$/ /$$/\  $$
      | $$ \______/ |__/  \__/ \_______/|_______/  \______/ |__/  \__/
 /$$  | $$                                                            
|  $$$$$$/                                                            
 \______/                                                             
"#;

    Text::raw(LOGO_TEXT).render(
        utils::align(
            Rect {
                width: 70,
                height: 13,
                ..area
            },
            area,
            utils::Alignment::Center,
        ),
        buf,
    );
}
