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
    pub fn new(media_controls: bool) -> Result<Self, String> {
        let (sender, receiver) = std::sync::mpsc::channel();

        let media_controls = if media_controls {
            Some(MediaControls::new(sender.clone())?)
        } else {
            None
        };

        Ok(Self {
            sender,
            receiver,
            media_controls,
        })
    }

    pub fn start(&self) {
        let sender = self.sender.clone();
        #[cfg(target_os = "windows")]
        let media_controls = self.media_controls.is_some();

        std::thread::spawn(move || {
            handle_terminal_events(
                sender,
                #[cfg(target_os = "windows")]
                media_controls,
            )
        });
    }

    pub fn next(&self) -> Result<Event, std::sync::mpsc::RecvError> {
        Ok(self.receiver.recv()?)
    }

    pub fn set_media(&mut self, title: &str, artist: &str, playback: MediaPlayback) {
        let Some(media) = self.media_controls.as_mut() else {
            return;
        };

        let _ = media.0.set_metadata(souvlaki::MediaMetadata {
            title: Some(title),
            artist: Some(artist),
            // TODO: cover_url?
            ..Default::default()
        });
        let _ = media.0.set_playback(playback.into());
    }

    pub fn set_playback(&mut self, playback: MediaPlayback) {
        let Some(media) = self.media_controls.as_mut() else {
            return;
        };

        let _ = media.0.set_playback(playback.into());
    }

    pub fn reset_media(&mut self) {
        let Some(media) = self.media_controls.as_mut() else {
            return;
        };

        let _ = media.0.set_metadata(souvlaki::MediaMetadata::default());
        let _ = media.0.set_playback(souvlaki::MediaPlayback::Stopped);
    }
}

fn handle_terminal_events(
    sender: Sender,
    #[cfg(target_os = "windows")] media_controls: bool,
) -> Result<(), std::io::Error> {
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

        #[cfg(target_os = "windows")]
        if media_controls {
            // this must be run repeatedly by your program to ensure
            // the Windows event queue is processed by your application
            windows::pump_event_queue();
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

impl From<MediaPlayback> for souvlaki::MediaPlayback {
    fn from(value: MediaPlayback) -> Self {
        match value {
            MediaPlayback::Playing => souvlaki::MediaPlayback::Playing { progress: None },
            MediaPlayback::Paused => souvlaki::MediaPlayback::Paused { progress: None },
            MediaPlayback::Stopped => souvlaki::MediaPlayback::Stopped,
        }
    }
}

impl MediaControls {
    fn new(sender: Sender) -> Result<Self, String> {
        #[cfg(not(target_os = "windows"))]
        let hwnd = None;

        #[cfg(target_os = "windows")]
        let (hwnd, _dummy_window) = {
            let dummy_window = windows::DummyWindow::new().unwrap();
            let handle = Some(dummy_window.handle.0 as _);
            (handle, dummy_window)
        };

        let config = souvlaki::PlatformConfig {
            display_name: crate::APP_NAME,
            // TODO: Add random number to avoid zbus panic when dbus name is already taken?
            // Currently a second instance of trollstov will panic.
            dbus_name: crate::symbols::concat!(
                crate::APP_QUALIFIER,
                ".",
                crate::APP_ORGANIZATION,
                ".",
                crate::APP_NAME
            ),
            hwnd, // TODO: Add proper Windows OS support.
        };

        let mut controls = souvlaki::MediaControls::new(config)
            .map_err(|err| format!("Failed to create media controls due to {}", err))?;

        controls
            .attach(move |event| {
                handle_media_events(event, &sender);
            })
            .map_err(|err| {
                format!(
                    "Failed to attach static handler \
            for media controls due to {}",
                    err
                )
            })?;

        // Wait a bit in case zbus panic due to dbus name taken
        // TODO: Remove when fixed: https://github.com/Sinono3/souvlaki/issues/32
        std::thread::sleep(Duration::from_millis(100));

        Ok(Self(controls))
    }
}

fn handle_media_events(event: souvlaki::MediaControlEvent, sender: &Sender) {
    let event = match event {
        souvlaki::MediaControlEvent::Play => MediaEvent::Play,
        souvlaki::MediaControlEvent::Pause => MediaEvent::Pause,
        souvlaki::MediaControlEvent::Toggle => MediaEvent::Toggle,
        souvlaki::MediaControlEvent::Next => MediaEvent::Next,
        souvlaki::MediaControlEvent::Previous => MediaEvent::Previous,
        souvlaki::MediaControlEvent::Stop => MediaEvent::Stop,
        souvlaki::MediaControlEvent::Raise => MediaEvent::Raise,
        souvlaki::MediaControlEvent::Quit => MediaEvent::Quit,
        _ => return,
    };
    let _ = sender.send(Event::Media(event));
}

// demonstrates how to make a minimal window to allow use of media keys on the command line
// https://github.com/Sinono3/souvlaki/blob/master/examples/print_events.rs
#[cfg(target_os = "windows")]
mod windows {
    use std::io::Error;
    use std::mem;

    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GA_ROOT, GetAncestor,
        IsDialogMessageW, MSG, PM_REMOVE, PeekMessageW, RegisterClassExW, TranslateMessage,
        WINDOW_EX_STYLE, WINDOW_STYLE, WM_QUIT, WNDCLASSEXW,
    };
    use windows::core::PCWSTR;
    use windows::w;

    pub struct DummyWindow {
        pub handle: HWND,
    }

    impl DummyWindow {
        pub fn new() -> Result<DummyWindow, String> {
            let class_name = w!("SimpleTray");

            let handle_result = unsafe {
                let instance = GetModuleHandleW(None)
                    .map_err(|e| (format!("Getting module handle failed: {e}")))?;

                let wnd_class = WNDCLASSEXW {
                    cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
                    hInstance: instance,
                    lpszClassName: PCWSTR::from(class_name),
                    lpfnWndProc: Some(Self::wnd_proc),
                    ..Default::default()
                };

                if RegisterClassExW(&wnd_class) == 0 {
                    return Err(format!(
                        "Registering class failed: {}",
                        Error::last_os_error()
                    ));
                }

                let handle = CreateWindowExW(
                    WINDOW_EX_STYLE::default(),
                    class_name,
                    w!(""),
                    WINDOW_STYLE::default(),
                    0,
                    0,
                    0,
                    0,
                    None,
                    None,
                    instance,
                    None,
                );

                if handle.0 == 0 {
                    Err(format!(
                        "Message only window creation failed: {}",
                        Error::last_os_error()
                    ))
                } else {
                    Ok(handle)
                }
            };

            handle_result.map(|handle| DummyWindow { handle })
        }
        extern "system" fn wnd_proc(
            hwnd: HWND,
            msg: u32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
        }
    }

    impl Drop for DummyWindow {
        fn drop(&mut self) {
            unsafe {
                DestroyWindow(self.handle);
            }
        }
    }

    pub fn pump_event_queue() -> bool {
        unsafe {
            let mut msg: MSG = std::mem::zeroed();
            let mut has_message = PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool();
            while msg.message != WM_QUIT && has_message {
                if !IsDialogMessageW(GetAncestor(msg.hwnd, GA_ROOT), &msg).as_bool() {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                has_message = PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool();
            }

            msg.message == WM_QUIT
        }
    }
}
