use std::sync::mpsc;

use crate::AudioFileReport;

pub struct MediaControls {
    controls: souvlaki::MediaControls,
    receiver: mpsc::Receiver<MediaEvent>,
}

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
    pub fn new(name: &str) -> Result<Self, AudioFileReport> {
        let config = souvlaki::PlatformConfig {
            display_name: name,
            dbus_name: name,
            hwnd: None,
        };
        let mut controls = souvlaki::MediaControls::new(config).map_err(|err| {
            AudioFileReport::new(format!(
                "Failed to create media controls for the Media Player \
                Remote Interfacing Specification (MPRIS) due to {}",
                err
            ))
        })?;

        let (sender, receiver) = mpsc::channel();
        controls
            .attach(move |event: souvlaki::MediaControlEvent| {
                match event {
                    souvlaki::MediaControlEvent::Play => {
                        let _ = sender.send(MediaEvent::Play);
                    }
                    souvlaki::MediaControlEvent::Pause => {
                        let _ = sender.send(MediaEvent::Pause);
                    }
                    souvlaki::MediaControlEvent::Toggle => {
                        let _ = sender.send(MediaEvent::Toggle);
                    }
                    souvlaki::MediaControlEvent::Next => {
                        let _ = sender.send(MediaEvent::Next);
                    }
                    souvlaki::MediaControlEvent::Previous => {
                        let _ = sender.send(MediaEvent::Previous);
                    }
                    souvlaki::MediaControlEvent::Stop => {
                        let _ = sender.send(MediaEvent::Stop);
                    }
                    souvlaki::MediaControlEvent::Raise => {
                        let _ = sender.send(MediaEvent::Raise);
                    }
                    souvlaki::MediaControlEvent::Quit => {
                        let _ = sender.send(MediaEvent::Quit);
                    }
                    _ => {}
                };
            })
            .map_err(|err| {
                AudioFileReport::new(format!(
                    "Failed to attach static handler for Media Player \
                    Remote Interfacing Specification (MPRIS) due to {}",
                    err
                ))
            })?;

        Ok(Self { controls, receiver })
    }

    pub fn try_recv(&self) -> Option<MediaEvent> {
        self.receiver.try_recv().ok()
    }

    pub fn set_metadata(&mut self, title: &str, artist: &str) {
        let _ = self.controls.set_metadata(souvlaki::MediaMetadata {
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
        let _ = self.controls.set_playback(playback);
    }

    pub fn reset_metadata(&mut self) {
        let _ = self
            .controls
            .set_metadata(souvlaki::MediaMetadata::default());
    }
}
