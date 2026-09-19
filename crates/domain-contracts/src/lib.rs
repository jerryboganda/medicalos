//! Shared domain contracts: the single source of truth for rules that must be
//! identical on the server, in Tauri apps, and in the web build (plan §20.1).
//! Built natively and for wasm32-unknown-unknown; CI proves both (ARCH-01).

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

pub const MIN_DAILY_STUDY_MINUTES: i32 = 1;
pub const MAX_DAILY_STUDY_MINUTES: i32 = 1440;
pub const MAX_PROTECTED_COMMITMENT_TITLE_LEN: usize = 120;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ProtectedCommitment {
    pub title: String,
    pub date: NaiveDate,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct GoalUpdate {
    pub expected_version: i32,
    pub daily_minutes: Option<i32>,
    pub exam_date: Option<NaiveDate>,
    #[serde(default)]
    pub protected_commitments: Vec<ProtectedCommitment>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum GoalValidationError {
    InvalidVersion,
    InvalidDailyMinutes,
    PastExamDate,
    InvalidCommitmentTitle,
    PastCommitmentDate,
}

pub fn validate_goal_version(version: i32) -> Result<(), GoalValidationError> {
    if version < 0 {
        return Err(GoalValidationError::InvalidVersion);
    }
    Ok(())
}

pub fn validate_goal_update(
    mut input: GoalUpdate,
    today: NaiveDate,
) -> Result<GoalUpdate, GoalValidationError> {
    validate_goal_version(input.expected_version)?;

    if matches!(input.daily_minutes, Some(minutes) if !(MIN_DAILY_STUDY_MINUTES..=MAX_DAILY_STUDY_MINUTES).contains(&minutes))
    {
        return Err(GoalValidationError::InvalidDailyMinutes);
    }

    if matches!(input.exam_date, Some(date) if date < today) {
        return Err(GoalValidationError::PastExamDate);
    }

    for commitment in &mut input.protected_commitments {
        commitment.title = commitment.title.trim().to_owned();
        if commitment.title.is_empty()
            || commitment.title.chars().count() > MAX_PROTECTED_COMMITMENT_TITLE_LEN
        {
            return Err(GoalValidationError::InvalidCommitmentTitle);
        }
        if commitment.date < today {
            return Err(GoalValidationError::PastCommitmentDate);
        }
    }

    Ok(input)
}

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

    #[test]
    fn validates_and_normalizes_goal_updates() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 19).unwrap();
        let validated = validate_goal_update(
            GoalUpdate {
                expected_version: 0,
                daily_minutes: Some(45),
                exam_date: Some(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()),
                protected_commitments: vec![ProtectedCommitment {
                    title: "  Hospital teaching day  ".to_owned(),
                    date: NaiveDate::from_ymd_opt(2026, 9, 25).unwrap(),
                }],
            },
            today,
        )
        .unwrap();

        assert_eq!(
            validated.protected_commitments[0].title,
            "Hospital teaching day"
        );
    }

    #[test]
    fn rejects_invalid_goal_constraints() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 19).unwrap();
        let invalid = GoalUpdate {
            expected_version: 0,
            daily_minutes: Some(0),
            exam_date: None,
            protected_commitments: Vec::new(),
        };

        assert_eq!(
            validate_goal_update(invalid, today),
            Err(GoalValidationError::InvalidDailyMinutes)
        );
    }
}
