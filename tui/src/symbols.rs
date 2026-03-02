pub const ALT: &str = "⎇";
pub const CTRL: &str = "^";
pub const SHIFT: &str = "⇧";
pub const ENTER: &str = "↵";
pub const SPACE: &str = "Space";
pub const ESCAPE: &str = "Esc";
pub const ARROW_UP: &str = "￪";
pub const ARROW_DOWN: &str = "￬";
pub const ARROW_RIGHT: &str = "→";
pub const ARROW_DOWN_UP: &str = "⇵";
pub const ARROW_LEFT_RIGHT: &str = "⇆";

macro_rules! alt {
    ($s:expr) => {{
        const _: &str = $s;
        constcat::concat!(crate::symbols::ALT, $s)
    }};
}

macro_rules! ctrl {
    ($s:expr) => {{
        const _: &str = $s;
        constcat::concat!(crate::symbols::CTRL, $s)
    }};
}

macro_rules! shift {
    ($s:expr) => {{
        const _: &str = $s;
        constcat::concat!("(", crate::symbols::SHIFT, ")", $s)
    }};
}

pub(crate) use alt;
pub(crate) use ctrl;
pub(crate) use shift;
