use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use color_eyre::eyre::WrapErr;
use ratatui::crossterm::event::{self, Event as CrosstermEvent};

use crate::pages::Route;

const UPDATE_FREQUENCY: f64 = 1.0 / 8.0;
const RENDER_FREQUENCY: f64 = 1.0 / 1.0;

pub enum Event {
    Terminal(CrosstermEvent),
    App(AppEvent),
}

pub enum AppEvent {
    Update,
    Render,
    Route(Route),
    Quit,
}

pub struct EventHandler {
    sender: mpsc::Sender<Event>,
    receiver: mpsc::Receiver<Event>,
}

impl EventHandler {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let actor = EventThread::new(sender.clone());
        thread::spawn(|| actor.run());
        Self { sender, receiver }
    }

    pub fn next(&self) -> color_eyre::Result<Event> {
        Ok(self.receiver.recv()?)
    }

    pub fn send(&self, app_event: AppEvent) {
        let _ = self.sender.send(Event::App(app_event));
    }
}

struct EventThread {
    sender: mpsc::Sender<Event>,
}

impl EventThread {
    fn new(sender: mpsc::Sender<Event>) -> Self {
        Self { sender }
    }

    fn run(self) -> color_eyre::Result<()> {
        // Setup timers
        let update_interval = Duration::from_secs_f64(UPDATE_FREQUENCY);
        let mut last_update = Instant::now();
        let render_interval = Duration::from_secs_f64(RENDER_FREQUENCY);
        let mut last_render = Instant::now();

        loop {
            // Update at a fixed rate
            let update_timeout = update_interval.saturating_sub(last_update.elapsed());
            if update_timeout == Duration::ZERO {
                last_update = Instant::now();
                self.send(Event::App(AppEvent::Update));
            }

            // Render at a fixed rate
            let render_timeout = render_interval.saturating_sub(last_render.elapsed());
            if render_timeout == Duration::ZERO {
                last_render = Instant::now();
                self.send(Event::App(AppEvent::Render));
            }

            // Poll for crossterm events in a non-blocking manner
            if event::poll(update_timeout).wrap_err("failed to poll for crossterm events")? {
                let event = event::read().wrap_err("failed to read crossterm event")?;
                self.send(Event::Terminal(event));
            }
        }
    }

    fn send(&self, event: Event) {
        let _ = self.sender.send(event);
    }
}
