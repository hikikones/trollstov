use std::time::Duration;

/// Formats an integer to its string representation.
pub fn format_int(i: impl itoa::Integer, mut f: impl FnMut(&str)) {
    let mut buffer = itoa::Buffer::new();
    f(buffer.format(i))
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
        chars[1] = unsafe { buffer.format(minutes).chars().next().unwrap_unchecked() };
    } else if minutes < 100 {
        for (i, char) in buffer.format(minutes).chars().enumerate() {
            chars[i] = char;
        }
    } else {
        return ['9', '9', ':', '9', '9'];
    }

    if seconds < 10 {
        chars[4] = unsafe { buffer.format(seconds).chars().next().unwrap_unchecked() };
    } else {
        for (i, char) in buffer.format(seconds).chars().enumerate() {
            chars[i + 3] = char;
        }
    }

    chars
}
