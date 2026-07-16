const MAX_MIDI_VALUE: u8 = 127;
const NUMERIC_INPUT_MAX_DIGITS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::tui) enum NumericInputTarget {
    CcNumber,
    CcValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::tui) struct NumericInput {
    target: NumericInputTarget,
    buffer: String,
}

impl NumericInput {
    pub(super) fn new(target: NumericInputTarget) -> Self {
        Self {
            target,
            buffer: String::new(),
        }
    }

    pub(in crate::tui) fn target(&self) -> NumericInputTarget {
        self.target
    }

    pub(in crate::tui) fn buffer(&self) -> &str {
        &self.buffer
    }

    pub(super) fn push(&mut self, digit: char) {
        if digit.is_ascii_digit() && self.buffer.len() < NUMERIC_INPUT_MAX_DIGITS {
            self.buffer.push(digit);
        }
    }

    pub(super) fn backspace(&mut self) {
        self.buffer.pop();
    }

    pub(super) fn confirmed_value(&self) -> Option<u8> {
        Some(self.buffer.parse::<u16>().ok()?.min(MAX_MIDI_VALUE as u16) as u8)
    }
}
