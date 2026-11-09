//! Log text helpers

/// Removes ANSI escape sequences (CSI and OSC) and stray control characters
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\x1b' => match chars.peek() {
                Some('[') => {
                    chars.next();
                    // parameters/intermediates until a final byte 0x40..=0x7e
                    for n in chars.by_ref() {
                        if ('\x40'..='\x7e').contains(&n) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    chars.next();
                    // until BEL or ST (ESC \)
                    while let Some(n) = chars.next() {
                        if n == '\x07' {
                            break;
                        }
                        if n == '\x1b' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                _ => {
                    chars.next();
                }
            },
            '\t' => out.push_str("    "),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// Rough level of a log line: "error", "warn", "ok" or "info"
pub fn classify(line: &str) -> &'static str {
    let l = line.to_ascii_lowercase();
    if l.contains("error") || l.contains("exception") || l.contains("failed") || l.contains("fatal") {
        "error"
    } else if l.contains("warn") {
        "warn"
    } else if l.contains("success") || l.contains("started webserver") || l.contains("server is running") || l.contains("done") {
        "ok"
    } else {
        "info"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips() {
        assert_eq!(strip_ansi("\x1b[1;32mOK\x1b[0m done"), "OK done");
        assert_eq!(strip_ansi("\x1b]0;title\x07x"), "x");
        assert_eq!(classify("Server is running, do not close"), "ok");
        assert_eq!(classify("[ERROR] boom"), "error");
    }
}
