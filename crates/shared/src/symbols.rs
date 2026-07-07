pub const TAB: &str = "Tab";
pub const ALT: &str = "⎇";
pub const CTRL: &str = "^";
pub const SHIFT: &str = "⇧";
pub const ENTER: &str = "↵";
pub const SPACE: &str = "Space";
pub const ESCAPE: &str = "Esc";
pub const DELETE: &str = "Del";
pub const ARROW_UP: &str = "￪";
pub const ARROW_DOWN: &str = "￬";
pub const ARROW_RIGHT: &str = "➔";
pub const ARROW_DOWN_UP: &str = "⇵";
pub const ARROW_RIGHT_LEFT: &str = "⇄";
pub const ARROW_LEFT_RIGHT: &str = "⇆";
pub const ARROW_HEAD_UP: &str = "⌃";
pub const ARROW_HEAD_DOWN: &str = "⌄";
pub const LINE_VERTICAL_LIGHT: &str = "│";
pub const LINE_VERTICAL_HEAVY: &str = "┃";
pub const LINE_HORIZONTAL_LIGHT: &str = "─";
pub const LINE_HORIZONTAL_LIGHT_CHAR: char = '─';
pub const LINE_HORIZONTAL_HEAVY: &str = "━";

pub const SELECTED: &str = ">";
pub const SELECTION: &str = "|";
pub const STAR: &str = "★";
pub const STAR_BIG: &str = "🟊";
pub const CHECKMARK_YES: &str = "🗸";
pub const CHECKMARK_NO: &str = "✗";

pub const fn checkmark(b: bool) -> &'static str {
    if b { CHECKMARK_YES } else { CHECKMARK_NO }
}

pub const fn checkmark_with_value<T: Copy>(b: bool, yes: T, no: T) -> (&'static str, T) {
    if b {
        (CHECKMARK_YES, yes)
    } else {
        (CHECKMARK_NO, no)
    }
}

#[macro_export]
macro_rules! alt {
    ($s:expr) => {{
        const _: &str = $s;
        shared::symbols::concat!(shared::symbols::ALT, $s)
    }};
}

#[macro_export]
macro_rules! ctrl {
    ($s:expr) => {{
        const _: &str = $s;
        shared::symbols::concat!(shared::symbols::CTRL, $s)
    }};
}

#[macro_export]
macro_rules! shift {
    ($s:expr) => {{
        const _: &str = $s;
        shared::symbols::concat!("(", shared::symbols::SHIFT, ")", $s)
    }};
}

#[macro_export]
macro_rules! repeat {
    ($s:expr, 0) => {{
        const _: &str = $s;
        ""
    }};
    ($s:expr, 1) => {{
        const _: &str = $s;
        $s
    }};
    ($s:expr, 2) => {{
        const _: &str = $s;
        shared::symbols::concat!($s, $s)
    }};
    ($s:expr, 3) => {{
        const _: &str = $s;
        shared::symbols::concat!($s, $s, $s)
    }};
    ($s:expr, 4) => {{
        const _: &str = $s;
        shared::symbols::concat!($s, $s, $s, $s)
    }};
    ($s:expr, 5) => {{
        const _: &str = $s;
        shared::symbols::concat!($s, $s, $s, $s, $s)
    }};
}

pub use alt;
pub use constcat::concat;
pub use ctrl;
pub use repeat;
pub use shift;
