//! SR-01/SR-02: spaced-repetition scheduling. The FSRS implementation is the
//! official rs-fsrs crate (MIT, license-reviewed per plan §13) — the same
//! scheduler runs on the server and on devices via WebAssembly (§20.1).
//!
//! This wrapper keeps the domain boundary: callers speak in plans and queues;
//! the FSRS math stays behind rs-fsrs. Daily caps and backlog triage (SR-02)
//! are deterministic rules here, unit-tested like all shared business rules.

use chrono::{DateTime, Utc};
pub use rs_fsrs::{Card, Rating, FSRS};

#[derive(Debug, Clone)]
pub struct Scheduler {
    fsrs: FSRS,
}

// FSRS::default() carries the default 17/19-parameter set — the transparent
// baseline of §8.4. Trained per-learner parameters arrive only through the
// §8.5 controlled-comparison path, never by silent retuning.
impl Default for Scheduler {
    fn default() -> Self {
        Scheduler {
            fsrs: FSRS::default(),
        }
    }
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_card(&self) -> Card {
        Card::new()
    }

    /// Apply one review: returns the next card state. Deterministic for the
    /// same inputs (SR-01), same result on server and devices.
    pub fn review(&self, card: Card, rating: Rating, now: DateTime<Utc>) -> Card {
        self.fsrs.repeat(card, now)[&rating].card.clone()
    }
}

/// SR-02 queue limits (plan §13 defaults: cap of 30 reviews/day adjustable;
/// new-card caps from §8.6 — default 10).
#[derive(Debug, Clone, Copy)]
pub struct QueueLimits {
    pub max_reviews_per_day: usize,
    pub max_new_per_day: usize,
}

impl Default for QueueLimits {
    fn default() -> Self {
        QueueLimits {
            max_reviews_per_day: 30,
            max_new_per_day: 10,
        }
    }
}

/// A card paired with its caller-side identity (deck item id).
#[derive(Debug, Clone)]
pub struct QueueCard {
    pub id: String,
    pub card: Card,
}

#[derive(Debug, Default)]
pub struct Queue {
    pub due: Vec<QueueCard>,
    pub new: Vec<QueueCard>,
    /// Due cards that did not fit today's cap — most-at-risk-first triage
    /// means the ones left behind are the least overdue (§13).
    pub backlog_remaining: usize,
}

/// Build today's study queue. Due cards come first (oldest due = most at
/// risk), then new cards up to the new-card cap. # ponytail: risk ordering
/// uses due-date age as the retrievability proxy; swap to the FSRS
/// retrievability value when the review-history store lands (Phase 2 SR
/// slice continues there).
pub fn build_queue(limits: QueueLimits, due: Vec<QueueCard>, new_cards: Vec<QueueCard>) -> Queue {
    let mut due = due;
    due.sort_by(|a, b| a.card.due.cmp(&b.card.due));
    let queued_due = due.len().min(limits.max_reviews_per_day);
    let backlog_remaining = due.len() - queued_due;
    Queue {
        due: due.into_iter().take(queued_due).collect(),
        new: new_cards.into_iter().take(limits.max_new_per_day).collect(),
        backlog_remaining,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_card_review_moves_due_into_the_future() {
        let s = Scheduler::new();
        let now = Utc::now();
        let card = s.new_card();
        let reviewed = s.review(card, Rating::Good, now);
        assert!(
            reviewed.due > now,
            "a Good review schedules the card forward"
        );
    }

    #[test]
    fn again_schedules_shorter_than_good() {
        let s = Scheduler::new();
        let now = Utc::now();
        let base = s.review(s.new_card(), Rating::Good, now);
        let after_good = s.review(base.clone(), Rating::Good, now);
        let after_again = s.review(base, Rating::Again, now);
        assert!(
            after_again.due < after_good.due,
            "lapses come back sooner than successful recall"
        );
    }

    #[test]
    fn queue_respects_daily_caps_and_counts_backlog() {
        let s = Scheduler::new();
        let limits = QueueLimits {
            max_reviews_per_day: 5,
            max_new_per_day: 2,
        };
        let due: Vec<QueueCard> = (0..8)
            .map(|i| QueueCard {
                id: format!("d{i}"),
                card: s.new_card(),
            })
            .collect();
        let fresh: Vec<QueueCard> = (0..6)
            .map(|i| QueueCard {
                id: format!("n{i}"),
                card: s.new_card(),
            })
            .collect();
        let queue = build_queue(limits, due, fresh);
        assert_eq!(queue.due.len(), 5, "review cap enforced");
        assert_eq!(queue.new.len(), 2, "new-card cap enforced");
        assert_eq!(queue.backlog_remaining, 3, "overflow counted, not dropped");
    }

    #[test]
    fn queue_is_empty_when_nothing_is_due() {
        let queue = build_queue(QueueLimits::default(), vec![], vec![]);
        assert!(queue.due.is_empty() && queue.new.is_empty());
        assert_eq!(queue.backlog_remaining, 0);
    }
}
