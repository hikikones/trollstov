use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event as CrosstermEvent};

type Sender = std::sync::mpsc::Sender<Event>;
type Receiver = std::sync::mpsc::Receiver<Event>;

pub enum Event {
    Update,
    Render,
    Media(MediaEvent),
    Terminal(CrosstermEvent),
}

pub struct EventHandler {
    sender: Sender,
    receiver: Receiver,
    media_controls: Option<MediaControls>,
}

impl EventHandler {
    pub fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        Self {
            sender,
            receiver,
            media_controls: None,
        }
    }

    pub fn start(&self) {
        std::thread::spawn({
            let sender = self.sender.clone();
            move || handle_events(sender)
        });
    }

    pub fn next(&self) -> Result<Event, std::sync::mpsc::RecvError> {
        Ok(self.receiver.recv()?)
    }

    pub fn try_establish_media_controls(
        &mut self,
        dbus_name: &str,
        display_name: &str,
    ) -> Result<(), String> {
        let config = souvlaki::PlatformConfig {
            display_name,
            dbus_name,
            hwnd: None,
        };

        let mut controls = souvlaki::MediaControls::new(config).map_err(|err| {
            format!(
                "Failed to create media controls for the Media Player \
                Remote Interfacing Specification (MPRIS) due to {}",
                err
            )
        })?;

        controls
            .attach({
                let sender = self.sender.clone();
                move |event| {
                    handle_media_control_event(event, &sender);
                }
            })
            .map_err(|err| {
                format!(
                    "Failed to attach static handler for Media Player \
                    Remote Interfacing Specification (MPRIS) due to {}",
                    err
                )
            })?;

        self.media_controls = Some(MediaControls(controls));

        Ok(())
    }

    pub fn media_controls(&mut self) -> Option<&mut MediaControls> {
        self.media_controls.as_mut()
    }
}

fn handle_events(sender: Sender) -> Result<(), std::io::Error> {
    const UPDATE_FREQUENCY: f64 = 1.0 / 8.0;
    const RENDER_FREQUENCY: f64 = 1.0 / 1.0;

    // Setup timers
    let mut update = Timer::new(Duration::from_secs_f64(UPDATE_FREQUENCY));
    let mut render = Timer::new(Duration::from_secs_f64(RENDER_FREQUENCY));

    loop {
        // Update at a fixed rate
        if update.tick() {
            let _ = sender.send(Event::Update);
        }

        // Render at a fixed rate
        if render.tick() {
            let _ = sender.send(Event::Render);
        }

        // Poll for crossterm events in a non-blocking manner
        if event::poll(update.timeout())? {
            let event = event::read()?;
            let _ = sender.send(Event::Terminal(event));
        }
    }
}

struct Timer {
    interval: Duration,
    last_tick: Instant,
    timeout: Duration,
}

impl Timer {
    fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_tick: Instant::now(),
            timeout: Duration::ZERO,
        }
    }

    fn tick(&mut self) -> bool {
        self.timeout = self.interval.saturating_sub(self.last_tick.elapsed());
        if self.timeout == Duration::ZERO {
            self.last_tick = Instant::now();
            true
        } else {
            false
        }
    }

    const fn timeout(&self) -> Duration {
        self.timeout
    }
}

fn handle_media_control_event(event: souvlaki::MediaControlEvent, sender: &Sender) {
    let event = match event {
        souvlaki::MediaControlEvent::Play => MediaEvent::Play,
        souvlaki::MediaControlEvent::Pause => MediaEvent::Pause,
        souvlaki::MediaControlEvent::Toggle => MediaEvent::Toggle,
        souvlaki::MediaControlEvent::Next => MediaEvent::Next,
        souvlaki::MediaControlEvent::Previous => MediaEvent::Previous,
        souvlaki::MediaControlEvent::Stop => MediaEvent::Stop,
        souvlaki::MediaControlEvent::Raise => MediaEvent::Raise,
        souvlaki::MediaControlEvent::Quit => MediaEvent::Quit,
        _ => {
            return;
        }
    };
    let _ = sender.send(Event::Media(event));
}

pub struct MediaControls(souvlaki::MediaControls);

pub enum MediaEvent {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    Stop,
    Raise,
    Quit,
}

pub enum MediaPlayback {
    Playing,
    Paused,
    Stopped,
}

impl MediaControls {
    pub fn set_metadata(&mut self, title: &str, artist: &str) {
        let _ = self.0.set_metadata(souvlaki::MediaMetadata {
            title: Some(title),
            artist: Some(artist),
            // TODO: cover_url?
            ..Default::default()
        });
    }

    pub fn set_playback(&mut self, playback: MediaPlayback) {
        let playback = match playback {
            MediaPlayback::Playing => souvlaki::MediaPlayback::Playing { progress: None },
            MediaPlayback::Paused => souvlaki::MediaPlayback::Paused { progress: None },
            MediaPlayback::Stopped => souvlaki::MediaPlayback::Stopped,
        };
        let _ = self.0.set_playback(playback);
    }

    pub fn reset_metadata(&mut self) {
        let _ = self.0.set_metadata(souvlaki::MediaMetadata::default());
    }
}
