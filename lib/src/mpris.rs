use std::sync::mpsc;

use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, PlatformConfig};

pub(super) struct Mpris {
    controls: MediaControls,
    receiver: mpsc::Receiver<MprisEvent>,
}

pub(super) enum MprisEvent {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    Stop,
}

impl Mpris {
    pub(super) fn new() -> Result<Self, souvlaki::Error> {
        let config = PlatformConfig {
            display_name: "jukebox",
            dbus_name: "jukebox",
            hwnd: None,
        };
        let mut controls = MediaControls::new(config)?;

        let (sender, receiver) = mpsc::channel();
        controls.attach(move |event: MediaControlEvent| {
            match event {
                MediaControlEvent::Play => {
                    let _ = sender.send(MprisEvent::Play);
                }
                MediaControlEvent::Pause => {
                    let _ = sender.send(MprisEvent::Pause);
                }
                MediaControlEvent::Toggle => {
                    let _ = sender.send(MprisEvent::Toggle);
                }
                MediaControlEvent::Next => {
                    let _ = sender.send(MprisEvent::Next);
                }
                MediaControlEvent::Previous => {
                    let _ = sender.send(MprisEvent::Previous);
                }
                MediaControlEvent::Stop => {
                    let _ = sender.send(MprisEvent::Stop);
                }
                _ => {}
            };
        })?;

        Ok(Self { controls, receiver })
    }

    pub(super) fn try_recv(&self) -> Option<MprisEvent> {
        self.receiver.try_recv().ok()
    }

    pub(super) fn set_metadata(&mut self, title: &str, artist: &str) {
        let _ = self.controls.set_metadata(MediaMetadata {
            title: Some(title),
            artist: Some(artist),
            ..Default::default()
        });
    }
}
