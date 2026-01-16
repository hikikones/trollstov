use std::io::stdout;

use ratatui::{
    CompletedFrame, DefaultTerminal, Frame,
    crossterm::{
        event::{
            KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
        },
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    prelude::CrosstermBackend,
};

pub struct Terminal {
    terminal: DefaultTerminal,
    clear: bool,
}

impl Terminal {
    pub fn init() -> std::io::Result<Self> {
        set_panic_hook();
        enable_raw_mode()?;

        let mut stdout = stdout();

        execute!(
            stdout,
            EnterAlternateScreen,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;

        let backend = CrosstermBackend::new(stdout);

        Ok(Self {
            terminal: DefaultTerminal::new(backend)?,
            clear: false,
        })
    }

    pub fn restore() -> std::io::Result<()> {
        disable_raw_mode()?;

        execute!(stdout(), PopKeyboardEnhancementFlags, LeaveAlternateScreen)?;

        Ok(())
    }

    pub fn draw<F>(&mut self, render_callback: F) -> std::io::Result<CompletedFrame<'_>>
    where
        F: FnOnce(&mut Frame),
    {
        if self.clear {
            self.terminal.clear()?;
            self.clear = false;
        }

        self.terminal.try_draw(|frame| {
            render_callback(frame);
            std::io::Result::Ok(())
        })
    }

    pub fn _temp_leave<T>(&mut self, f: impl FnOnce() -> std::io::Result<T>) -> std::io::Result<T> {
        let mut stdout = std::io::stdout();

        execute!(stdout, LeaveAlternateScreen)?;
        disable_raw_mode()?;

        let t = f();

        execute!(stdout, EnterAlternateScreen)?;
        enable_raw_mode()?;

        self.clear = true;

        t
    }
}

fn set_panic_hook() {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        hook(info);
    }));
}
