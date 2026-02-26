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
    colors::Colors,
    events::{Event, EventHandler},
    pages::{Log, Pages, Route},
    terminal::Terminal,
    widgets::{Shortcut, Shortcuts, TextSegment, utils},
};

// TODO: Add a dynamic playlist page for artists/albums/genres and filtering.

const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

type FrontCoverHandle = std::thread::JoinHandle<Result<FrontCover, AudioFileReport>>;

pub struct App {
    running: bool,
    pages: Pages,
    route: Route,
    colors: Colors,
    events: EventHandler,
    jukebox: Jukebox,
    mpris: bool,
    picker: Picker,
    screen_size: ScreenSize,
    front_cover: FrontCover,
    front_cover_handle: Option<FrontCoverHandle>,
    text_segment: TextSegment,
    shortcuts_page: Shortcuts,
    shortcuts_play: Shortcuts,
    shortcuts_app: Shortcuts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenSize {
    Small,
    Medium,
    Large,
}

impl ScreenSize {
    const fn from_rect(area: Rect) -> ScreenSize {
        match (area.width, area.height) {
            (w, h) if w < 68 || h < 20 => ScreenSize::Small,
            (w, h) if w < 108 || h < 30 => ScreenSize::Medium,
            _ => ScreenSize::Large,
        }
    }
}

pub enum FrontCover {
    None,
    Loading,
    Ready(StatefulProtocol),
}

pub enum Action {
    None,
    Render,
    Route(Route),
    Quit,
}

impl App {
    pub fn new(jukebox: Jukebox, colors: Colors, picker: Picker, mpris: bool) -> Self {
        let pages = Pages::new(&colors);

        let shortcuts_page = Shortcuts::new(Color::Reset, colors.accent);
        let shortcuts_play = Shortcuts::new(colors.neutral, colors.accent);
        let shortcuts_app = Shortcuts::new(colors.neutral, colors.accent);

        Self {
            running: true,
            pages,
            route: Route::default(),
            colors,
            events: EventHandler::new(),
            jukebox,
            mpris,
            picker,
            screen_size: ScreenSize::Large,
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
            frame.render_widget(crate::widgets::LogoWidget, frame.area());
        })?;

        // Start reading events and load music
        self.events.start();
        self.jukebox.load_music();

        // Try to establish media controls
        if self.mpris {
            match self.jukebox.attach_media_controls(APP_NAME) {
                Ok(_) => {
                    self.mpris = true;
                }
                Err(err) => {
                    self.mpris = false;
                    self.pages.logs.enqueue(Log::new(err));
                }
            }
        }

        self.on_enter();

        // Run event loop
        while self.running {
            let action = match self.events.next()? {
                Event::Update => self.update(),
                Event::Render => Action::Render,
                Event::Terminal(event) => match event {
                    CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => {
                        self.handle_key_press(key)
                    }
                    _ => Action::None,
                },
            };

            match action {
                Action::None => {}
                Action::Render => {
                    self.render(&mut terminal)?;
                }
                Action::Route(route) => {
                    self.on_exit();
                    self.route = route;
                    self.on_enter();
                    self.render(&mut terminal)?;
                }
                Action::Quit => {
                    self.running = false;
                }
            }
        }

        Ok(())
    }

    pub fn quit(self) {
        self.jukebox.shutdown();
    }

    fn handle_key_press(&mut self, key: KeyEvent) -> Action {
        let alt = key.modifiers.contains(KeyModifiers::ALT);
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        match key.code {
            KeyCode::Esc => {
                return Action::Quit;
            }
            KeyCode::Tab => {
                return Action::Route(self.route.next());
            }
            KeyCode::BackTab => {
                return Action::Route(self.route.prev());
            }
            KeyCode::Up => {
                if ctrl {
                    self.jukebox.pause_or_play();
                } else if alt {
                    let new_volume = (self.jukebox.volume() + 0.1).min(2.0);
                    self.jukebox.set_volume(new_volume);
                    return Action::Render;
                } else {
                    return self.on_input(key);
                }
            }
            KeyCode::Down => {
                if ctrl {
                    self.jukebox.stop();
                } else if alt {
                    let new_volume = (self.jukebox.volume() - 0.1).max(0.0);
                    self.jukebox.set_volume(new_volume);
                    return Action::Render;
                } else {
                    return self.on_input(key);
                }
            }
            KeyCode::Right => {
                if ctrl {
                    self.jukebox.play_next();
                } else if alt {
                    self.jukebox.fast_forward_by(Duration::from_secs(30));
                    return Action::Render;
                } else {
                    return self.on_input(key);
                }
            }
            KeyCode::Left => {
                if ctrl {
                    self.jukebox.play_previous();
                } else {
                    return self.on_input(key);
                }
            }
            KeyCode::Media(media) => {
                // Ignore when we have media controls through MPRIS
                if !self.mpris {
                    match media {
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
                    }
                }
            }
            KeyCode::Char(c) => match c {
                '/' => {
                    if self.route == Route::Search {
                        return self.on_input(key);
                    } else {
                        self.pages.search.set_search();
                        return Action::Route(Route::Search);
                    }
                }
                _ => {
                    return self.on_input(key);
                }
            },
            _ => {
                return self.on_input(key);
            }
        }

        Action::None
    }

    fn update(&mut self) -> Action {
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
                            const MAX_RES: u32 = 1080;
                            let mut dyn_img = image::load_from_memory(bytes).map_err(|err| {
                                AudioFileReport::new(format!(
                                    "Failed to load front cover image for \"{}\" due to {}",
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
                self.pages.logs.enqueue(Log::new(err));
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
                        self.pages.logs.enqueue(Log::new(err));
                        self.front_cover = FrontCover::None;
                    }
                }
            }
        }

        if render { Action::Render } else { Action::None }
    }

    fn render<'a>(&'a mut self, terminal: &'a mut Terminal) -> std::io::Result<CompletedFrame<'a>> {
        terminal.draw(|frame| {
            let area = frame.area();
            let buf = frame.buffer_mut();

            self.shortcuts_page.clear();
            self.shortcuts_play.clear();
            self.shortcuts_app.clear();
            self.screen_size = ScreenSize::from_rect(area);

            const MARGIN: u16 = 1;

            match self.screen_size {
                ScreenSize::Small => {
                    // Body
                    self.on_render(area, buf);
                }
                ScreenSize::Medium => {
                    // Layout
                    let [
                        nav_area,
                        body_area,
                        shortcuts_page_area,
                        _,
                        playback_area,
                        shortcuts_play_area,
                        shortcuts_app_area,
                    ] = Layout::vertical([
                        Constraint::Length(1),
                        Constraint::Min(5),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(2),
                        Constraint::Length(1),
                        Constraint::Length(1),
                    ])
                    .areas(area);

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
                    let body = body_area.inner(Margin::new(MARGIN, MARGIN));
                    self.on_render(body, buf);
                    self.shortcuts_page.render(shortcuts_page_area, buf);

                    // Playback
                    render_playback(
                        playback_area,
                        buf,
                        &mut self.text_segment,
                        self.jukebox.current_track_pos(),
                        self.jukebox
                            .current_track_id()
                            .and_then(|id| self.jukebox.get(id)),
                        &self.colors,
                    );

                    // Shortcuts
                    fill_play_shortcuts(&mut self.shortcuts_play, self.jukebox.volume());
                    fill_app_shortcuts(&mut self.shortcuts_app);
                    self.shortcuts_play.render(shortcuts_play_area, buf);
                    self.shortcuts_app.render(shortcuts_app_area, buf);
                }
                ScreenSize::Large => {
                    // Layout
                    let [
                        title_area,
                        _,
                        nav_area,
                        body_area,
                        shortcuts_page_area,
                        _,
                        playback_area,
                        shortcuts_app_area,
                    ] = Layout::vertical([
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Min(5),
                        Constraint::Length(1),
                        Constraint::Length(1),
                        Constraint::Length(2),
                        Constraint::Length(1),
                    ])
                    .areas(area);

                    // Title
                    utils::print_ascii_iter(
                        title_area,
                        buf,
                        &[APP_NAME, " ", APP_VERSION],
                        self.colors.neutral,
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
                    const MAX_WIDTH: u16 = 160;
                    let body = body_area
                        .centered_horizontally(Constraint::Length(MAX_WIDTH + MARGIN))
                        .inner(Margin::new(MARGIN, MARGIN));
                    self.on_render(body, buf);
                    self.shortcuts_page.render(shortcuts_page_area, buf);

                    // Playback
                    render_playback(
                        playback_area,
                        buf,
                        &mut self.text_segment,
                        self.jukebox.current_track_pos(),
                        self.jukebox
                            .current_track_id()
                            .and_then(|id| self.jukebox.get(id)),
                        &self.colors,
                    );

                    // Shortcuts
                    fill_app_shortcuts(&mut self.shortcuts_app);
                    fill_play_shortcuts(&mut self.shortcuts_app, self.jukebox.volume());
                    self.shortcuts_app.render(shortcuts_app_area, buf);
                }
            }
        })
    }

    fn on_render(&mut self, body: Rect, buf: &mut Buffer) {
        match self.route {
            Route::Tracks(_) => {
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
                    self.screen_size,
                    &mut self.front_cover,
                    &self.colors,
                    &mut self.shortcuts_page,
                );
            }
            Route::Search => {
                self.pages.search.on_render(
                    body,
                    buf,
                    &mut self.jukebox,
                    &self.colors,
                    &mut self.shortcuts_page,
                );
            }
            Route::Logs => {
                self.pages
                    .logs
                    .on_render(body, buf, &self.colors, &mut self.shortcuts_page);
            }
        }
    }

    fn on_enter(&mut self) {
        match self.route {
            Route::Tracks(id) => self.pages.tracks.on_enter(id, &self.jukebox),
            Route::NowPlaying => self.pages.playing.on_enter(),
            Route::Search => self.pages.search.on_enter(),
            Route::Logs => self.pages.logs.on_enter(),
        }
    }

    fn on_exit(&mut self) {
        match self.route {
            Route::Tracks(_) => self.pages.tracks.on_exit(),
            Route::NowPlaying => self.pages.playing.on_exit(),
            Route::Search => self.pages.search.on_exit(),
            Route::Logs => self.pages.logs.on_exit(),
        }
    }

    fn on_input(&mut self, key: KeyEvent) -> Action {
        match self.route {
            Route::Tracks(_) => {
                self.pages
                    .tracks
                    .on_input(key.code, key.modifiers, &mut self.jukebox)
            }
            Route::NowPlaying => self.pages.playing.on_input(
                key.code,
                key.modifiers,
                &mut self.jukebox,
                self.screen_size,
            ),
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
        (Route::Tracks(None), "Tracks", SPACING),
        (Route::NowPlaying, "Now Playing", SPACING),
        (Route::Search, "Search", SPACING),
        (Route::Logs, "Logs", ""),
    ] {
        let is_current = std::mem::discriminant(&route) == std::mem::discriminant(&current_route);
        let style = if is_current {
            Style::new().fg(colors.accent).bold()
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

fn render_playback(
    area: Rect,
    buf: &mut Buffer,
    text: &mut TextSegment,
    audio_position: Duration,
    track: Option<&Track>,
    colors: &Colors,
) {
    let accent = Style::new().fg(colors.accent);
    let neutral = Style::new().fg(colors.neutral);

    let title_line = Rect { height: 1, ..area };
    let status_line = Rect {
        y: area.y + 1,
        ..title_line
    };

    let status_width = (0.64 * status_line.width as f32).ceil() as u16;
    let progress_ch = '─';
    let progress_highlight_ch = '━';

    match track {
        Some(track) => {
            // Title
            text.extend_as_one(["[", track.extension().as_upper_case()], neutral);

            if let Some(bit_depth) = track.bit_depth()
                && let Some(sample_rate) = track.sample_rate()
            {
                jukebox::utils::format_int(bit_depth, |bit_depth| {
                    text.extend_as_one([" ", bit_depth, "bit/"], neutral);
                });
                jukebox::utils::format_int(sample_rate, |sample_rate| {
                    text.extend_as_one([sample_rate, "kHz"], neutral);
                });
            }

            jukebox::utils::format_int(track.bit_rate(), |bit_rate| {
                text.extend_as_one([" ", bit_rate, "kbps] "], neutral);
            });

            text.push_str(track.artist(), neutral);
            if !(track.artist().is_empty() || track.title().is_empty()) {
                text.push_str(" - ", neutral);
            }
            text.push_str(track.title(), neutral);

            text.render(title_line, buf);
            text.clear();

            // Status
            text.push_chars(
                &jukebox::utils::format_duration_on_stack(audio_position),
                neutral,
            );
            text.push_char(' ', Style::new());

            let progress = audio_position.as_secs_f32() / track.duration().as_secs_f32();
            let max_highlight_bound = (status_width as f32 * progress) as u16;
            for i in 0..status_width {
                let (ch, style) = if i <= max_highlight_bound {
                    (progress_highlight_ch, accent)
                } else {
                    (progress_ch, neutral)
                };
                text.push_char(ch, style);
            }

            text.push_char(' ', Style::new());
            text.push_chars(
                &jukebox::utils::format_duration_on_stack(track.duration()),
                neutral,
            );
        }
        None => {
            text.push_str("00:00 ", neutral);
            text.repeat_char(progress_ch, status_width as usize, neutral);
            text.push_str(" 00:00", neutral);
        }
    }

    text.render(status_line, buf);
    text.clear();
}

fn fill_play_shortcuts(shortcuts: &mut Shortcuts, volume: f32) {
    shortcuts.extend([
        Shortcut::new("Play/Pause", "^￪"),
        Shortcut::new("Next/Prev", "^⇆"),
        Shortcut::new("Stop", "^￬"),
        Shortcut::new("Forward 30s", "⎇→"),
    ]);

    let volume = (volume * 100.0).round() as u8;
    jukebox::utils::format_int(volume, |volume| {
        shortcuts.push_iter(["Volume ", volume, "%"], "⎇⇵");
    });
}

fn fill_app_shortcuts(shortcuts: &mut Shortcuts) {
    shortcuts.extend([
        Shortcut::new("Quit", "Esc"),
        Shortcut::new("Navigate", "(⇧)Tab"),
        Shortcut::new("Search", "/"),
    ]);
}
