//! Shared domain contracts: the single source of truth for rules that must be
//! identical on the server, in Tauri apps, and in the web build (plan §20.1).
//! Built natively and for wasm32-unknown-unknown; CI proves both (ARCH-01).

use serde::{Deserialize, Serialize};

/// Number of answer options on a question. The exam-pack contract allows two to
/// ten options with generated labels (plan §11.1, QB-11); anything else is a
/// content-model error, not a runtime fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionCount(u8);

#[derive(Debug, PartialEq, Eq)]
pub struct OptionCountOutOfRange;

impl OptionCount {
    pub const MIN: u8 = 2;
    pub const MAX: u8 = 10;

    pub fn value(self) -> u8 {
        self.0
    }

    /// Generated labels per §11.5: 'labels generated' — A..J for 2..=10 options.
    pub fn labels(self) -> Vec<String> {
        (0..self.0)
            .map(|i| ((b'A' + i) as char).to_string())
            .collect()
    }
}

impl TryFrom<u8> for OptionCount {
    type Error = OptionCountOutOfRange;

    fn try_from(n: u8) -> Result<Self, Self::Error> {
        if (Self::MIN..=Self::MAX).contains(&n) {
            Ok(OptionCount(n))
        } else {
            Err(OptionCountOutOfRange)
        }
    }
}

/// Validated option count from a plain usize length (0..=255 inputs).
pub fn option_count(len: usize) -> Result<OptionCount, OptionCountOutOfRange> {
    let n = u8::try_from(len).map_err(|_| OptionCountOutOfRange)?;
    OptionCount::try_from(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_two_through_ten() {
        assert_eq!(OptionCount::try_from(2).unwrap().value(), 2);
        assert_eq!(OptionCount::try_from(10).unwrap().value(), 10);
    }

    #[test]
    fn rejects_out_of_range() {
        assert!(OptionCount::try_from(1).is_err());
        assert!(OptionCount::try_from(11).is_err());
        assert!(OptionCount::try_from(0).is_err());
    }

    #[test]
    fn generates_labels() {
        let labels = OptionCount::try_from(5).unwrap().labels();
        assert_eq!(labels, vec!["A", "B", "C", "D", "E"]);
    }
}
