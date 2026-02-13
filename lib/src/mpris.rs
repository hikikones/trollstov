use std::sync::mpsc;

pub(super) struct MediaControls {
    controls: souvlaki::MediaControls,
    receiver: mpsc::Receiver<MediaEvent>,
}

pub(super) enum MediaEvent {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
    Stop,
}

impl MediaControls {
    pub(super) fn new() -> Result<Self, souvlaki::Error> {
        let config = souvlaki::PlatformConfig {
            display_name: "jukebox",
            dbus_name: "jukebox",
            hwnd: None,
        };
        let mut controls = souvlaki::MediaControls::new(config)?;

        let (sender, receiver) = mpsc::channel();
        controls.attach(move |event: souvlaki::MediaControlEvent| {
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
                _ => {}
            };
        })?;

        Ok(Self { controls, receiver })
    }

    pub(super) fn try_recv(&self) -> Option<MediaEvent> {
        self.receiver.try_recv().ok()
    }

    pub(super) fn set_metadata(&mut self, title: &str, artist: &str) {
        let _ = self.controls.set_metadata(souvlaki::MediaMetadata {
            title: Some(title),
            artist: Some(artist),
            ..Default::default()
        });
    }
}
