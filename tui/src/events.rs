use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use ratatui::crossterm::event::{self, Event as CrosstermEvent};

const UPDATE_FREQUENCY: f64 = 1.0 / 8.0;
const RENDER_FREQUENCY: f64 = 1.0 / 1.0;

pub enum Event {
    Update,
    Render,
    Terminal(CrosstermEvent),
}

pub struct EventHandler {
    sender: mpsc::Sender<Event>,
    receiver: mpsc::Receiver<Event>,
}

impl EventHandler {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { sender, receiver }
    }

    pub fn start(&self) {
        let actor = EventThread(self.sender.clone());
        thread::spawn(|| actor.run());
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        Ok(self.receiver.recv()?)
    }
}

struct EventThread(mpsc::Sender<Event>);

impl EventThread {
    fn run(self) -> Result<(), std::io::Error> {
        // Setup timers
        let mut update = Timer::new(Duration::from_secs_f64(UPDATE_FREQUENCY));
        let mut render = Timer::new(Duration::from_secs_f64(RENDER_FREQUENCY));

        loop {
            // Update at a fixed rate
            if update.tick() {
                self.send(Event::Update);
            }

            // Render at a fixed rate
            if render.tick() {
                self.send(Event::Render);
            }

            // Poll for crossterm events in a non-blocking manner
            if event::poll(update.timeout())? {
                let event = event::read()?;
                self.send(Event::Terminal(event));
            }
        }
    }

    fn send(&self, event: Event) {
        let _ = self.0.send(event);
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
