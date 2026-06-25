//! Line-based text input buffer used by `INPUT`/`LINPUT`.
//!
//! Headless runners and tests preload completed lines via [`InputBuffer::push_line`].
//! Interactive hosts (e.g. the browser player) type into [`InputBuffer::current`] one
//! code point at a time and commit with [`InputBuffer::enter`].

use std::collections::VecDeque;

use crate::value::SbStr;

/// A line-oriented input queue for the `INPUT`/`LINPUT` statements.
///
/// Completed lines live in `lines`; the line currently being typed lives in `current`.
/// `INPUT` consumes from `lines`; when it is empty the host yields and waits for the user
/// to finish typing.
#[derive(Debug, Clone, Default)]
pub struct InputBuffer {
    lines: VecDeque<SbStr>,
    current: SbStr,
}

impl InputBuffer {
    /// Create an empty buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queue a completed line from a Rust string (headless/test path).
    pub fn push_line(&mut self, line: &str) {
        self.lines.push_back(line.encode_utf16().collect());
    }

    /// Append one Unicode code point to the line currently being typed.
    /// Astral code points are encoded as a UTF-16 surrogate pair.
    pub fn char(&mut self, ch: u32) {
        if ch <= 0xFFFF {
            self.current.push(ch as u16);
        } else if ch <= 0x10FFFF {
            let ch = ch - 0x1_0000;
            self.current.push(0xD800 + ((ch >> 10) as u16));
            self.current.push(0xDC00 + ((ch & 0x3FF) as u16));
        }
    }

    /// Append a whole Rust string to the current line.
    pub fn push_str(&mut self, s: &str) {
        self.current.extend(s.encode_utf16());
    }

    /// Remove the last code point from the current line. Handles surrogate pairs.
    pub fn backspace(&mut self) {
        if let Some(last) = self.current.pop() {
            // If we removed a low surrogate, also remove its high surrogate.
            if (0xDC00..=0xDFFF).contains(&last) {
                if let Some(prev) = self.current.last() {
                    if (0xD800..=0xDBFF).contains(prev) {
                        self.current.pop();
                    }
                }
            }
        }
    }

    /// Commit the current line as a completed input line.
    pub fn enter(&mut self) {
        let mut line = SbStr::new();
        std::mem::swap(&mut line, &mut self.current);
        self.lines.push_back(line);
    }

    /// Take the oldest completed line, if any.
    pub fn next_line(&mut self) -> Option<SbStr> {
        self.lines.pop_front()
    }

    /// Reference the line currently being typed (not yet submitted).
    pub fn current(&self) -> &SbStr {
        &self.current
    }

    /// Replace the current line. Used by hosts that let the browser handle IME/paste
    /// and mirror the final string into the VM.
    pub fn set_current(&mut self, line: &str) {
        self.current.clear();
        self.current.extend(line.encode_utf16());
    }

    /// Whether a completed line is available for `INPUT`/`LINPUT` to consume.
    pub fn has_line(&self) -> bool {
        !self.lines.is_empty()
    }

    /// Clear both queued lines and the current line.
    pub fn clear(&mut self) {
        self.lines.clear();
        self.current.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_line_and_next_line() {
        let mut b = InputBuffer::new();
        b.push_line("hello");
        b.push_line("world");
        assert_eq!(
            b.next_line().unwrap(),
            "hello".encode_utf16().collect::<Vec<_>>()
        );
        assert_eq!(
            b.next_line().unwrap(),
            "world".encode_utf16().collect::<Vec<_>>()
        );
        assert!(b.next_line().is_none());
    }

    #[test]
    fn char_and_enter() {
        let mut b = InputBuffer::new();
        for c in "hi".chars() {
            b.char(c as u32);
        }
        b.enter();
        assert_eq!(
            b.next_line().unwrap(),
            vec![u16::from(b'h'), u16::from(b'i')]
        );
    }

    #[test]
    fn backspace_removes_ascii() {
        let mut b = InputBuffer::new();
        b.push_str("abc");
        b.backspace();
        assert_eq!(b.current(), &"ab".encode_utf16().collect::<Vec<_>>());
    }

    #[test]
    fn backspace_removes_astral_codepoint() {
        let mut b = InputBuffer::new();
        b.push_str("a🎹b");
        b.backspace();
        assert_eq!(b.current(), &"a🎹".encode_utf16().collect::<Vec<_>>());
        b.backspace();
        assert_eq!(b.current(), &"a".encode_utf16().collect::<Vec<_>>());
    }

    #[test]
    fn astral_char_is_surrogate_pair() {
        let mut b = InputBuffer::new();
        b.char('🎹' as u32);
        assert_eq!(b.current().len(), 2);
        b.enter();
        assert_eq!(
            b.next_line().unwrap(),
            "🎹".encode_utf16().collect::<Vec<_>>()
        );
    }

    #[test]
    fn set_current_replaces() {
        let mut b = InputBuffer::new();
        b.push_str("old");
        b.set_current("new");
        assert_eq!(b.current(), &"new".encode_utf16().collect::<Vec<_>>());
    }
}
