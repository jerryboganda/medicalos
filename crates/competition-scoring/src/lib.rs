//! Competition scoring per plan §17.1: difficulty points (5/10/15,
//! configurable), wrong-answer penalty (25% of item points, so random
//! guessing never pays), speed bonus only on correct answers and capped at
//! 20% of item points (accuracy always dominates), and the tie-break ladder
//! score -> accuracy -> total time -> earlier submission.
//!
//! Deterministic and shared: the same crate will run on the server (live
//! events) and inside clients via WebAssembly (§20.1).

/// # ponytail: the speed-bonus window is a flat config constant; per-question
/// time budgets arrive with the competition module (Phase 4, §17.1 live
/// events) if match data shows the flat window distorts rankings.
#[derive(Debug, Clone)]
pub struct ScoringConfig {
    pub easy_points: i64,
    pub medium_points: i64,
    pub hard_points: i64,
    /// Fraction of item points deducted for a wrong answer (§17.1: 0.25).
    pub wrong_penalty_fraction: f64,
    /// Maximum speed bonus as a fraction of item points (§17.1: 0.20).
    pub speed_bonus_fraction: f64,
    /// Elapsed time (ms) at which the linear speed bonus reaches zero.
    pub speed_bonus_window_ms: i64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        ScoringConfig {
            easy_points: 5,
            medium_points: 10,
            hard_points: 15,
            wrong_penalty_fraction: 0.25,
            speed_bonus_fraction: 0.20,
            speed_bonus_window_ms: 20_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

#[derive(Debug, Clone, Copy)]
pub struct AnswerRecord {
    pub difficulty: Difficulty,
    pub correct: bool,
    /// Time from question shown to answer received.
    pub elapsed_ms: i64,
}

impl ScoringConfig {
    pub fn item_points(&self, difficulty: Difficulty) -> i64 {
        match difficulty {
            Difficulty::Easy => self.easy_points,
            Difficulty::Medium => self.medium_points,
            Difficulty::Hard => self.hard_points,
        }
    }

    /// Points for one answer: penalty if wrong; base + capped speed bonus if
    /// correct. Bonus decays linearly to zero across the configured window.
    pub fn item_score(&self, answer: &AnswerRecord) -> f64 {
        let base = self.item_points(answer.difficulty) as f64;
        if !answer.correct {
            return -(base * self.wrong_penalty_fraction);
        }
        let window = self.speed_bonus_window_ms.max(1) as f64;
        let remaining = (window - answer.elapsed_ms as f64).max(0.0) / window;
        base + base * self.speed_bonus_fraction * remaining
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    /// Stable participant identity (handle rendering is a display concern).
    pub participant_id: String,
    pub answers: Vec<AnswerRecord>,
    pub total_time_ms: i64,
    /// Submission sequence: smaller = earlier (tie-break of last resort).
    pub submitted_order: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    pub participant_id: String,
    pub score: f64,
    pub correct: usize,
    pub attempted: usize,
    pub accuracy: f64,
    pub total_time_ms: i64,
    pub submitted_order: i64,
}

fn accuracy(entry: &Entry) -> f64 {
    let attempted = entry.answers.len();
    if attempted == 0 {
        return 0.0;
    }
    let correct = entry.answers.iter().filter(|a| a.correct).count();
    correct as f64 / attempted as f64
}

/// Score every entry and rank it: score desc, then accuracy desc, then total
/// time asc, then earlier submission first (§17.1 tie-breaks).
pub fn rank(config: &ScoringConfig, entries: &[Entry]) -> Vec<Ranked> {
    let mut ranked: Vec<Ranked> = entries
        .iter()
        .map(|entry| {
            let score = entry
                .answers
                .iter()
                .map(|a| config.item_score(a))
                .sum::<f64>();
            let correct = entry.answers.iter().filter(|a| a.correct).count();
            Ranked {
                participant_id: entry.participant_id.clone(),
                score: (score * 100.0).round() / 100.0,
                correct,
                attempted: entry.answers.len(),
                accuracy: accuracy(entry),
                total_time_ms: entry.total_time_ms,
                submitted_order: entry.submitted_order,
            }
        })
        .collect();
    ranked.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(core::cmp::Ordering::Equal)
            .then(
                b.accuracy
                    .partial_cmp(&a.accuracy)
                    .unwrap_or(core::cmp::Ordering::Equal),
            )
            .then(a.total_time_ms.cmp(&b.total_time_ms))
            .then(a.submitted_order.cmp(&b.submitted_order))
    });
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(difficulty: Difficulty, correct: bool, elapsed_ms: i64) -> AnswerRecord {
        AnswerRecord {
            difficulty,
            correct,
            elapsed_ms,
        }
    }

    #[test]
    fn base_points_and_penalty_match_the_spec() {
        let cfg = ScoringConfig::default();
        // Correct, instant: base + 20% cap (speed bonus max).
        assert_eq!(cfg.item_score(&answer(Difficulty::Easy, true, 0)), 6.0);
        assert_eq!(cfg.item_score(&answer(Difficulty::Medium, true, 0)), 12.0);
        assert_eq!(cfg.item_score(&answer(Difficulty::Hard, true, 0)), 18.0);
        // Wrong: 25% penalty, guessing never pays.
        assert_eq!(cfg.item_score(&answer(Difficulty::Hard, false, 0)), -3.75);
    }

    #[test]
    fn speed_bonus_decays_to_zero_inside_the_window() {
        let cfg = ScoringConfig::default();
        // Half the window: half the bonus (10 * 0.20 * 0.5 = 1.0).
        assert_eq!(
            cfg.item_score(&answer(Difficulty::Medium, true, 10_000)),
            11.0
        );
        // At or past the window: base only.
        assert_eq!(
            cfg.item_score(&answer(Difficulty::Medium, true, 20_000)),
            10.0
        );
        assert_eq!(
            cfg.item_score(&answer(Difficulty::Medium, true, 999_999)),
            10.0
        );
    }

    #[test]
    fn accuracy_beats_speed_on_tied_scores() {
        let cfg = ScoringConfig::default();
        // Both end at the same score (5 base, no bonus in play) but the
        // faster perfect-scorer must lose the tie-break to higher accuracy.
        let fast_two_wrong = Entry {
            participant_id: "fast".into(),
            answers: vec![
                answer(Difficulty::Easy, false, 0),
                answer(Difficulty::Easy, false, 0),
                answer(Difficulty::Easy, true, 0),
                answer(Difficulty::Easy, true, 0),
            ], // 2*5 - 2*1.25 = 7.5
            total_time_ms: 1_000,
            submitted_order: 1,
        };
        let slow_three_of_three = Entry {
            participant_id: "slow".into(),
            answers: vec![
                answer(Difficulty::Easy, true, 20_000),
                answer(Difficulty::Easy, true, 20_000),
                answer(Difficulty::Easy, true, 20_000),
            ], // 3*5 = 15
            total_time_ms: 60_000,
            submitted_order: 2,
        };
        let ranked = rank(&cfg, &[fast_two_wrong, slow_three_of_three]);
        assert_eq!(ranked[0].participant_id, "slow");
        assert_eq!(ranked[0].score, 15.0);
    }

    #[test]
    fn full_tie_break_ladder() {
        let cfg = ScoringConfig::default();
        let mk = |id: &str, score_answers: Vec<AnswerRecord>, time: i64, order: i64| Entry {
            participant_id: id.into(),
            answers: score_answers,
            total_time_ms: time,
            submitted_order: order,
        };
        // a and b tie on score and accuracy; c loses on score.
        let a = mk(
            "a",
            vec![
                answer(Difficulty::Easy, true, 20_000),
                answer(Difficulty::Hard, false, 0),
            ],
            5_000,
            2,
        ); // 5 - 3.75 = 1.25
        let b = mk(
            "b",
            vec![
                answer(Difficulty::Easy, true, 20_000),
                answer(Difficulty::Hard, false, 0),
            ],
            4_000,
            3,
        ); // same score, same accuracy, faster
        let c = mk("c", vec![answer(Difficulty::Easy, false, 0)], 1_000, 1);
        let ranked = rank(&cfg, &[c.clone(), b.clone(), a.clone()]);
        assert_eq!(
            ranked[0].participant_id, "b",
            "faster total time wins the score tie"
        );
        assert_eq!(ranked[1].participant_id, "a");
        assert_eq!(ranked[2].participant_id, "c", "lower score ranks last");
        // Identical everything except submission order: earlier wins.
        let a2 = mk(
            "a2",
            vec![
                answer(Difficulty::Easy, true, 20_000),
                answer(Difficulty::Hard, false, 0),
            ],
            4_000,
            4,
        );
        let ranked = rank(&cfg, &[a2, b.clone()]);
        assert_eq!(
            ranked[0].participant_id, "b",
            "earlier submission wins the final tie"
        );
    }
}
