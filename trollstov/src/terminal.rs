use ratatui::{
    CompletedFrame, DefaultTerminal, Frame,
    backend::CrosstermBackend,
    crossterm::{
        event::{
            KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
        },
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
};

pub struct Terminal(DefaultTerminal);

impl Terminal {
    pub fn init() -> std::io::Result<Self> {
        set_panic_hook();
        enable_raw_mode()?;

        let mut stdout = std::io::stdout();

        execute!(
            stdout,
            EnterAlternateScreen,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;

        let backend = CrosstermBackend::new(stdout);

        Ok(Self(DefaultTerminal::new(backend)?))
    }

    pub fn restore() -> std::io::Result<()> {
        disable_raw_mode()?;

        execute!(
            std::io::stdout(),
            PopKeyboardEnhancementFlags,
            LeaveAlternateScreen
        )?;

        Ok(())
    }

    pub fn draw<F>(&mut self, render_callback: F) -> std::io::Result<CompletedFrame<'_>>
    where
        F: FnOnce(&mut Frame),
    {
        self.0.try_draw(|frame| {
            render_callback(frame);
            std::io::Result::Ok(())
        })
    }
}

fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Err(err) = Terminal::restore() {
            std::eprintln!("Failed to restore terminal: {err}");
        }
        hook(info);
    }));
}
