use std::time::Duration;

/// Formats an integer to its string representation.
pub fn format_int(i: impl itoa::Integer, mut f: impl FnMut(&str)) {
    let mut buffer = itoa::Buffer::new();
    f(buffer.format(i))
}

/// Formats two integers to its string representations.
pub fn format_int2(i1: impl itoa::Integer, i2: impl itoa::Integer, mut f: impl FnMut(&str, &str)) {
    let (mut b1, mut b2) = (itoa::Buffer::new(), itoa::Buffer::new());
    f(b1.format(i1), b2.format(i2))
}

/// Formats the duration as `mm:ss` to a String.
pub fn format_duration(duration: Duration) -> String {
    let mut s = String::with_capacity(5);
    format_duration_in_place(duration, &mut s);
    s
}

/// Formats the duration as `mm:ss` and appends it to the mutable String.
pub fn format_duration_in_place(duration: Duration, s: &mut String) {
    let seconds = duration.as_secs() % 60;
    let minutes = (duration.as_secs() - seconds) / 60;

    let mut buffer = itoa::Buffer::new();

    if minutes < 10 {
        s.push('0');
        s.push_str(buffer.format(minutes));
    } else if minutes < 100 {
        s.push_str(buffer.format(minutes));
    } else {
        s.push_str("99:99");
        return;
    }

    s.push(':');

    if seconds < 10 {
        s.push('0');
        s.push_str(buffer.format(seconds));
    } else {
        s.push_str(buffer.format(seconds));
    }
}

/// Formats the duration as `mm:ss` and returns a stack-allocated char array.
pub fn format_duration_on_stack(duration: Duration) -> [char; 5] {
    let seconds = duration.as_secs() % 60;
    let minutes = (duration.as_secs() - seconds) / 60;

    let mut buffer = itoa::Buffer::new();
    let mut chars = ['0', '0', ':', '0', '0'];

    if minutes < 10 {
        chars[1] = buffer.format(minutes).chars().next().unwrap();
    } else if minutes < 100 {
        for (i, char) in buffer.format(minutes).chars().enumerate() {
            chars[i] = char;
        }
    } else {
        return ['9', '9', ':', '9', '9'];
    }

    if seconds < 10 {
        chars[4] = buffer.format(seconds).chars().next().unwrap();
    } else {
        for (i, char) in buffer.format(seconds).chars().enumerate() {
            chars[i + 3] = char;
        }
    }

    chars
}

#[derive(Debug)]
pub struct Formatter(String);

impl Formatter {
    pub const fn new() -> Self {
        Self(String::new())
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }

    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }

    pub fn slice(&self, range: std::ops::Range<usize>) -> &str {
        &self.0[range]
    }

    pub fn push_str(&mut self, s: &str) -> std::ops::Range<usize> {
        let start = self.0.len();
        self.0.push_str(s);
        start..self.0.len()
    }

    pub fn push_str2(
        &mut self,
        s1: &str,
        s2: &str,
    ) -> (std::ops::Range<usize>, std::ops::Range<usize>) {
        let start = self.0.len();
        self.0.push_str(s1);
        let middle = self.0.len();
        self.0.push_str(s2);
        (start..middle, middle..self.0.len())
    }

    pub fn push_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::ops::Range<usize> {
        use std::fmt::Write;

        let start = self.0.len();
        let _ = self.0.write_fmt(args);
        start..self.0.len()
    }

    pub fn push_fmt2(
        &mut self,
        args1: std::fmt::Arguments<'_>,
        args2: std::fmt::Arguments<'_>,
    ) -> (std::ops::Range<usize>, std::ops::Range<usize>) {
        use std::fmt::Write;

        let start = self.0.len();
        let _ = self.0.write_fmt(args1);
        let middle = self.0.len();
        let _ = self.0.write_fmt(args2);
        (start..middle, middle..self.0.len())
    }

    pub fn extend<'a>(
        &mut self,
        iter: impl IntoIterator<Item = &'a str>,
    ) -> std::ops::Range<usize> {
        let start = self.0.len();
        self.0.extend(iter);
        start..self.0.len()
    }

    pub fn clear(&mut self) {
        self.0.clear();
    }
}

impl std::fmt::Display for Formatter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
