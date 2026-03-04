use std::{path::PathBuf, time::Duration};

use image::GenericImageView;
use jukebox::{
    AudioFileReport, AudioPicture, Database, DatabaseEvent, Jukebox, JukeboxEvent, MediaControls,
    MediaEvent, Track,
};
use ratatui::{
    CompletedFrame,
    crossterm::event::{
        Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MediaKeyCode,
    },
    prelude::*,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};

use crate::{
    events::{Event, EventHandler},
    pages::{Log, LogsPage, Pages, PlayingPage, Route, SearchPage, SettingsPage, TracksPage},
    settings::{Colors, Settings},
    symbols,
    terminal::Terminal,
    widgets::{Shortcut, Shortcuts, TextSegment, utils},
};

// TODO: Add a dynamic playlist page for artists/albums/genres and filtering.

type FrontCoverHandle = std::thread::JoinHandle<Result<FrontCover, AudioFileReport>>;

pub struct App {
    running: bool,
    pages: Pages,
    route: Route,
    events: EventHandler,
    settings: Settings,
    database: Database,
    jukebox: Jukebox,
    mpris: Option<MediaControls>,
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
    Log(Log),
    ApplySettings,
    Quit,
}

impl App {
    pub fn new(database: Database, jukebox: Jukebox, picker: Picker, mpris: bool) -> Self {
        let mut logs = LogsPage::new();

        let settings = Settings::read()
            .inspect_err(|err| {
                let log = Log::new(err);
                logs.enqueue(log);
            })
            .unwrap_or_default();

        let media_controls = {
            if mpris {
                match MediaControls::new(crate::APP_NAME) {
                    Ok(media_controls) => Some(media_controls),
                    Err(err) => {
                        let log = Log::new(err);
                        logs.enqueue(log);
                        None
                    }
                }
            } else {
                None
            }
        };

        let pages = Pages {
            tracks: TracksPage::new(),
            playing: PlayingPage::new(),
            search: SearchPage::new(),
            settings: SettingsPage::new(&settings),
            logs,
        };

        Self {
            running: true,
            pages,
            route: Route::default(),
            events: EventHandler::new(),
            settings,
            database,
            jukebox,
            mpris: media_controls,
            picker,
            screen_size: ScreenSize::Large,
            front_cover: FrontCover::None,
            front_cover_handle: None,
            text_segment: TextSegment::new().with_alignment(Alignment::Center),
            shortcuts_page: Shortcuts::new(),
            shortcuts_play: Shortcuts::new(),
            shortcuts_app: Shortcuts::new(),
        }
    }

    pub fn run(&mut self, mut terminal: Terminal) -> Result<(), Box<dyn std::error::Error>> {
        // Draw logo
        terminal.draw(|frame| {
            let color = self.settings.neutral();
            frame.render_widget(crate::widgets::LogoWidget(color), frame.area());
        })?;

        // Apply settings, read events, load music and enter first page
        self.apply_settings();
        self.events.start();
        self.database.load();
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
                Action::Log(log) => {
                    self.pages.logs.enqueue(log);
                    self.render(&mut terminal)?;
                }
                Action::ApplySettings => {
                    self.apply_settings();
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
        self.database.shutdown();
    }

    const fn apply_settings(&mut self) {
        self.jukebox.set_skip(self.settings.skip_rating());
        self.pages
            .tracks
            .set_keep_on_sort(self.settings.keep_on_sort());
        self.pages
            .search
            .set_search_by_path(self.settings.search_by_path());
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
                    return self.stop();
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
                    self.jukebox.play_next(&self.database);
                } else if alt {
                    self.jukebox.fast_forward_by(Duration::from_secs(30));
                    return Action::Render;
                } else {
                    return self.on_input(key);
                }
            }
            KeyCode::Left => {
                if ctrl {
                    self.jukebox.play_previous(&self.database);
                } else {
                    return self.on_input(key);
                }
            }
            KeyCode::Media(media) => {
                // Ignore when we have media controls through MPRIS
                if self.mpris.is_none() {
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
                            return self.stop();
                        }
                        MediaKeyCode::TrackNext => {
                            self.jukebox.play_next(&self.database);
                        }
                        MediaKeyCode::TrackPrevious => {
                            self.jukebox.play_previous(&self.database);
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

    fn stop(&mut self) -> Action {
        if self.jukebox.stop() {
            self.front_cover = FrontCover::None;
            if let Some(mpris) = self.mpris.as_mut() {
                mpris.reset_metadata();
            }
            return Action::Render;
        }

        Action::None
    }

    fn update(&mut self) -> Action {
        let mut render = false;

        // Update database
        self.database.update(|event| {
            render = true;
            match event {
                DatabaseEvent::Rating(_) => {}
                DatabaseEvent::Error(err) => {
                    self.pages.logs.enqueue(Log::new(err));
                }
            }
        });

        // Check for media control events
        if let Some(event) = self.mpris.as_ref().and_then(|mpris| mpris.try_recv()) {
            match event {
                MediaEvent::Play => self.jukebox.play(),
                MediaEvent::Pause => self.jukebox.pause(),
                MediaEvent::Toggle => self.jukebox.pause_or_play(),
                MediaEvent::Next => self.jukebox.play_next(&self.database),
                MediaEvent::Previous => self.jukebox.play_previous(&self.database),
                MediaEvent::Stop => {
                    if let Action::Render = self.stop() {
                        render = true;
                    }
                }
                MediaEvent::Raise => {
                    // TODO: Focus terminal window.
                }
                MediaEvent::Quit => {
                    self.running = false;
                }
            }
        }

        // Update jukebox
        self.jukebox.update(&self.database, |event| {
            render = true;
            match event {
                JukeboxEvent::Play(id) => {
                    if let Some(track) = self.database.get(id) {
                        // Start loading front cover image
                        let path = track.path().to_path_buf();
                        let picker = self.picker.clone();
                        let handle = load_front_cover(path, picker);
                        self.front_cover_handle = Some(handle);
                        self.front_cover = FrontCover::Loading;

                        // Update metadata for mpris
                        if let Some(mpris) = self.mpris.as_mut() {
                            mpris.set_metadata(track.title(), track.artist());
                        }
                    }
                }
                JukeboxEvent::Error(err) => {
                    self.pages.logs.enqueue(Log::new(err));
                }
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

            let colors = &self.settings.colors().clone();

            self.shortcuts_page
                .set_colors(Color::Reset, colors.accent)
                .clear();
            self.shortcuts_play
                .set_colors(colors.neutral, colors.accent)
                .clear();
            self.shortcuts_app
                .set_colors(colors.neutral, colors.accent)
                .clear();

            const MARGIN: u16 = 1;
            self.screen_size = ScreenSize::from_rect(area);

            match self.screen_size {
                ScreenSize::Small => {
                    // Body
                    self.on_render(area, buf, colors);
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
                        colors.accent,
                    );

                    // Body
                    let body = body_area.inner(Margin::new(MARGIN, MARGIN));
                    self.on_render(body, buf, colors);
                    self.shortcuts_page.render(shortcuts_page_area, buf);

                    // Playback
                    render_playback(
                        playback_area,
                        buf,
                        &mut self.text_segment,
                        self.jukebox.current_track_pos(),
                        self.jukebox
                            .current_track_id()
                            .and_then(|id| self.database.get(id)),
                        colors.accent,
                        colors.neutral,
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
                    utils::print_asciis(
                        title_area,
                        buf,
                        [crate::APP_NAME, " v", crate::APP_VERSION],
                        colors.neutral,
                        Some(utils::Alignment::CenterHorizontal),
                    );

                    // Navigation
                    render_navigation(
                        nav_area,
                        buf,
                        &mut self.text_segment,
                        self.route,
                        &self.pages,
                        colors.accent,
                    );

                    // Body
                    const MAX_WIDTH: u16 = 160;
                    let body = body_area
                        .centered_horizontally(Constraint::Length(MAX_WIDTH + MARGIN))
                        .inner(Margin::new(MARGIN, MARGIN));
                    self.on_render(body, buf, colors);
                    self.shortcuts_page.render(shortcuts_page_area, buf);

                    // Playback
                    render_playback(
                        playback_area,
                        buf,
                        &mut self.text_segment,
                        self.jukebox.current_track_pos(),
                        self.jukebox
                            .current_track_id()
                            .and_then(|id| self.database.get(id)),
                        colors.accent,
                        colors.neutral,
                    );

                    // Shortcuts
                    fill_app_shortcuts(&mut self.shortcuts_app);
                    fill_play_shortcuts(&mut self.shortcuts_app, self.jukebox.volume());
                    self.shortcuts_app.render(shortcuts_app_area, buf);
                }
            }
        })
    }

    fn on_render(&mut self, body: Rect, buf: &mut Buffer, colors: &Colors) {
        match self.route {
            Route::Tracks(_) => {
                self.pages.tracks.on_render(
                    body,
                    buf,
                    &self.database,
                    &self.jukebox,
                    colors,
                    &mut self.shortcuts_page,
                );
            }
            Route::NowPlaying => {
                self.pages.playing.on_render(
                    body,
                    buf,
                    &self.database,
                    &self.jukebox,
                    self.screen_size,
                    &mut self.front_cover,
                    colors,
                    &mut self.shortcuts_page,
                );
            }
            Route::Search => {
                self.pages.search.on_render(
                    body,
                    buf,
                    &mut self.database,
                    &mut self.jukebox,
                    colors,
                    &mut self.shortcuts_page,
                );
            }
            Route::Settings => {
                self.pages
                    .settings
                    .on_render(body, buf, &self.settings, &mut self.shortcuts_page);
            }
            Route::Logs => {
                self.pages
                    .logs
                    .on_render(body, buf, colors, &mut self.shortcuts_page);
            }
        }
    }

    fn on_enter(&mut self) {
        match self.route {
            Route::Tracks(id) => self.pages.tracks.on_enter(id, &self.database),
            Route::NowPlaying => self.pages.playing.on_enter(),
            Route::Search => self.pages.search.on_enter(),
            Route::Settings => self.pages.settings.on_enter(),
            Route::Logs => self.pages.logs.on_enter(),
        }
    }

    fn on_exit(&mut self) {
        match self.route {
            Route::Tracks(_) => self.pages.tracks.on_exit(),
            Route::NowPlaying => self.pages.playing.on_exit(),
            Route::Search => self.pages.search.on_exit(),
            Route::Settings => self.pages.settings.on_exit(),
            Route::Logs => self.pages.logs.on_exit(),
        }
    }

    fn on_input(&mut self, key: KeyEvent) -> Action {
        match self.route {
            Route::Tracks(_) => self.pages.tracks.on_input(
                key.code,
                key.modifiers,
                &mut self.database,
                &mut self.jukebox,
            ),
            Route::NowPlaying => self.pages.playing.on_input(
                key.code,
                key.modifiers,
                &mut self.database,
                &mut self.jukebox,
                self.screen_size,
            ),
            Route::Search => self.pages.search.on_input(
                key.code,
                key.modifiers,
                &self.database,
                &mut self.jukebox,
            ),
            Route::Settings => {
                self.pages
                    .settings
                    .on_input(key.code, key.modifiers, &mut self.settings)
            }
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
    accent: Color,
) {
    const SPACING: &str = "   ";
    for (route, name, spacing) in [
        (Route::Tracks(None), "Tracks", SPACING),
        (Route::NowPlaying, "Now Playing", SPACING),
        (Route::Search, "Search", SPACING),
        (Route::Settings, "Settings", SPACING),
        (Route::Logs, "Logs", ""),
    ] {
        let is_current = std::mem::discriminant(&route) == std::mem::discriminant(&current_route);
        let style = if is_current {
            Style::new().fg(accent).bold()
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
    accent: Color,
    neutral: Color,
) {
    let accent = Style::new().fg(accent);
    let neutral = Style::new().fg(neutral);

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
        Shortcut::new("Play/Pause", symbols::ctrl!(symbols::ARROW_UP)),
        Shortcut::new("Next/Prev", symbols::ctrl!(symbols::ARROW_LEFT_RIGHT)),
        Shortcut::new("Stop", symbols::ctrl!(symbols::ARROW_DOWN)),
        Shortcut::new("Forward 30s", symbols::ctrl!(symbols::ARROW_RIGHT)),
    ]);

    let volume = (volume * 100.0).round() as u8;
    jukebox::utils::format_int(volume, |volume| {
        shortcuts.push_iter(
            ["Volume ", volume, "%"],
            symbols::alt!(symbols::ARROW_DOWN_UP),
        );
    });
}

fn fill_app_shortcuts(shortcuts: &mut Shortcuts) {
    shortcuts.extend([
        Shortcut::new("Quit", symbols::ESCAPE),
        Shortcut::new("Navigate", symbols::shift!("Tab")),
        Shortcut::new("Search", "/"),
    ]);
}

fn load_front_cover(path: PathBuf, picker: Picker) -> FrontCoverHandle {
    std::thread::spawn(move || {
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
    })
}
