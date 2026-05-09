// SPDX-License-Identifier: Unlicense
//! Memorization-grade feedback consumer.
//!
//! The classifier today is a hard-coded keyword tree — there's no
//! model state to update. But the user's accumulated yes/no decisions
//! in `~/.atsisbroken/feedback.jsonl` still encode preference: if the
//! user said "no" to filling Greenhouse's "How did you hear about us?"
//! field once, they probably don't want it autofilled on the next
//! posting either.
//!
//! `CorrectionOverlay` summarizes the feedback ledger into a fingerprint
//! → most-recent-decision map. The run loop consults it BEFORE the
//! mode-specific prompt: if the user previously rejected this exact
//! field shape, skip without prompting.
//!
//! When R4 lands a trained classifier, this overlay stays useful as a
//! short-term memorization layer that the slower retrain step trails.

use crate::{Feedback, FieldDescriptor};
use std::collections::HashMap;

/// What the user decided about a particular field shape last time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserDecision {
    /// User accepted the classifier's call. Fill without prompting.
    Accepted,
    /// User rejected the classifier's call. Skip without prompting.
    Rejected,
}

/// Stable identity of a form field across runs / postings / tenants.
/// Deliberately EXCLUDES the DOM `id` because Workday and similar
/// vendors generate dynamic ids (`input-1234567890-<uuid>`) that
/// change per page load. Includes everything else the user can see
/// or that the form author hand-picked.
/// Stable fingerprint of a field descriptor for the
/// CorrectionOverlay key. `pub(crate)` so the tests tree can pin
/// the normalization (case-fold, whitespace-collapse, drop the
/// volatile `id` attribute).
pub(crate) fn fingerprint(d: &FieldDescriptor) -> String {
    format!(
        "{}|{}|{}|{}|{}",
        d.label.trim().to_lowercase(),
        d.placeholder.trim().to_lowercase(),
        d.aria_label.trim().to_lowercase(),
        d.name.trim().to_lowercase(),
        d.kind.trim().to_lowercase(),
    )
}

#[derive(Debug, Clone, Default)]
pub struct CorrectionOverlay {
    decisions: HashMap<String, UserDecision>,
}

impl CorrectionOverlay {
    /// Build from a feedback queue. **Most-recent decision wins** —
    /// the queue is iterated in order so a later "yes" overrides an
    /// earlier "no" for the same fingerprint.
    ///
    /// Every event with `accepted=true` enters as `Accepted`; every
    /// `accepted=false` enters as `Rejected`. Chaos auto-fills and
    /// TrainingWheels "y" responses currently look identical on the
    /// wire (both `accepted=true, predicted==actual`); we can't
    /// distinguish them without a schema change. Treating them
    /// uniformly means: a Chaos user who lets a field auto-fill is
    /// implicitly graduating that field shape to "auto" for future
    /// TrainingWheels runs too. That matches user intent (you don't
    /// downgrade modes by accident).
    pub fn from_queue(events: &[Feedback]) -> Self {
        let mut decisions: HashMap<String, UserDecision> = HashMap::new();
        for ev in events {
            let decision = if ev.accepted {
                UserDecision::Accepted
            } else {
                UserDecision::Rejected
            };
            decisions.insert(fingerprint(&ev.field), decision);
        }
        Self { decisions }
    }

    /// What did the user decide about this field shape last time, if
    /// they've seen it. None ⇒ no prior signal; defer to mode policy.
    pub fn decision_for(&self, d: &FieldDescriptor) -> Option<UserDecision> {
        self.decisions.get(&fingerprint(d)).copied()
    }

    pub fn len(&self) -> usize {
        self.decisions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.decisions.is_empty()
    }
}

/// Summary stats for the `feedback` subcommand. Read-only; no model
/// state mutated.
#[derive(Debug, Clone, Default)]
pub struct FeedbackStats {
    pub total: usize,
    pub accepted: usize,
    pub rejected: usize,
    /// Per-predicted-key acceptance counts.
    pub per_key_accepted: HashMap<String, usize>,
    pub per_key_rejected: HashMap<String, usize>,
    /// Most rejected (predicted_key, count). Sorted desc by count.
    pub top_rejected_keys: Vec<(String, usize)>,
}

impl FeedbackStats {
    pub fn from_queue(events: &[Feedback]) -> Self {
        let mut s = FeedbackStats {
            total: events.len(),
            ..Default::default()
        };
        for ev in events {
            if ev.accepted {
                s.accepted += 1;
                *s.per_key_accepted.entry(ev.predicted.clone()).or_insert(0) += 1;
            } else {
                s.rejected += 1;
                *s.per_key_rejected.entry(ev.predicted.clone()).or_insert(0) += 1;
            }
        }
        let mut sorted: Vec<(String, usize)> = s
            .per_key_rejected
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(5);
        s.top_rejected_keys = sorted;
        s
    }
}

