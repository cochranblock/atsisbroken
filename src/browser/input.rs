// SPDX-License-Identifier: Unlicense

//! Per-character human-like input. The browser dispatches real
//! keyboard events with Gaussian-jittered timing, mouse-focuses
//! before typing, and pauses between fields proportional to
//! field complexity. This is the core of the "looks human"
//! property.
//!
//! No `rand` dep — we use a small splitmix64 PRNG seeded once per
//! session. Deterministic given a seed (tests pin one). The threat
//! model is: keep the inter-keystroke timing distribution from
//! looking obviously machine-generated to a vendor's casual
//! anti-bot heuristic that bins event deltas. splitmix64 covers
//! that — its output is statistically uniform and fools per-event
//! distribution checks. It is NOT cryptographically secure: a
//! vendor that captures enough timing samples and runs an adversarial
//! analysis can recover the seed and predict subsequent delays. If
//! that becomes the relevant threat, swap in `rand::rngs::OsRng`
//! or a CSPRNG; the rest of the timing code is distribution-shape
//! agnostic.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// How fast the user types and with what variance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanInputProfile {
    /// Mean inter-character delay in milliseconds. ~100 ms ≈ 100
    /// WPM, which is "fast typist." Lower means faster than real
    /// humans (the goal — slightly faster). Higher means more
    /// natural for slower typists.
    pub mean_char_delay_ms: f32,
    /// Standard deviation in ms around the mean. Keeps the
    /// distribution Gaussian-ish; humans aren't uniform-random.
    /// 30 ms is realistic for a fluent typist.
    pub char_delay_stddev_ms: f32,
    /// Inter-field "thinking pause" mean in ms. Time between
    /// finishing one field and clicking the next. Default 350 ms.
    pub mean_field_pause_ms: f32,
    pub field_pause_stddev_ms: f32,
    /// Dwell time on `keydown` before `keyup` for each key.
    /// Real humans hold keys ~50-80 ms. Affects the
    /// `KeyboardEvent.timeStamp` deltas the page sees.
    pub key_dwell_ms: f32,
    /// Mouse movement: time to traverse the screen in ms.
    /// Bezier-curved, not straight-line.
    pub mouse_move_duration_ms: f32,
    /// PRNG seed for reproducibility. Non-zero for prod; tests
    /// pin to known seeds.
    pub seed: u64,
    /// Distribution shape — Gaussian is natural, Uniform is
    /// mostly for tests.
    pub timing_distribution: KeyTimingDistribution,
}

impl Default for HumanInputProfile {
    fn default() -> Self {
        Self {
            mean_char_delay_ms: 100.0,
            char_delay_stddev_ms: 30.0,
            mean_field_pause_ms: 350.0,
            field_pause_stddev_ms: 120.0,
            key_dwell_ms: 60.0,
            mouse_move_duration_ms: 220.0,
            // RNG seed for timing jitter — deterministic per-
            // session by default so screenshots reproduce. Users
            // override via `--input-seed` or by editing config.
            seed: 0xA75_15_B30_FAB1ED5,
            timing_distribution: KeyTimingDistribution::Gaussian,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KeyTimingDistribution {
    /// Box-Muller Gaussian. Realistic.
    Gaussian,
    /// Uniform over [mean - stddev, mean + stddev]. Used in tests
    /// where Gaussian's tails make assertions flaky.
    Uniform,
    /// Fixed delay = mean exactly. Mostly for headless screenshot
    /// reproducibility.
    Fixed,
}

/// Stateful per-session timing generator.
pub struct InputTiming {
    profile: HumanInputProfile,
    rng_state: u64,
}

impl InputTiming {
    pub fn new(profile: HumanInputProfile) -> Self {
        let seed = profile.seed;
        Self {
            profile,
            rng_state: seed,
        }
    }

    /// Sample one inter-character delay.
    pub fn next_char_delay(&mut self) -> Duration {
        self.sample(self.profile.mean_char_delay_ms, self.profile.char_delay_stddev_ms)
    }

    /// Sample one inter-field pause.
    pub fn next_field_pause(&mut self) -> Duration {
        self.sample(self.profile.mean_field_pause_ms, self.profile.field_pause_stddev_ms)
    }

    /// Per-key dwell (held-down time). Less variance than typing
    /// rhythm; mostly fixed.
    pub fn key_dwell(&self) -> Duration {
        Duration::from_secs_f32(self.profile.key_dwell_ms / 1000.0)
    }

    pub fn mouse_move_duration(&self) -> Duration {
        Duration::from_secs_f32(self.profile.mouse_move_duration_ms / 1000.0)
    }

    fn sample(&mut self, mean_ms: f32, stddev_ms: f32) -> Duration {
        let raw = match self.profile.timing_distribution {
            KeyTimingDistribution::Fixed => mean_ms,
            KeyTimingDistribution::Uniform => {
                let u = self.next_uniform();
                mean_ms - stddev_ms + u * 2.0 * stddev_ms
            }
            KeyTimingDistribution::Gaussian => {
                let z = self.next_gaussian();
                mean_ms + z * stddev_ms
            }
        };
        // Clamp to a non-negative floor. Sub-millisecond inputs
        // collapse to 1ms; we never dispatch with zero delay
        // (that would defeat the human-like property).
        let clamped = raw.max(1.0);
        Duration::from_secs_f32(clamped / 1000.0)
    }

    /// splitmix64 — small, fast, good enough for timing jitter.
    /// Source: Sebastiano Vigna's splitmix64, public domain.
    fn next_u64(&mut self) -> u64 {
        self.rng_state = self.rng_state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.rng_state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// Uniform [0, 1).
    fn next_uniform(&mut self) -> f32 {
        // Use the top 24 bits to fill an f32 mantissa.
        let bits = (self.next_u64() >> 40) as u32;
        bits as f32 / (1u32 << 24) as f32
    }

    /// Box-Muller: two uniforms → one standard-normal sample.
    /// We discard the second sample for simplicity; not memory-
    /// efficient but correctness-equivalent for our use.
    fn next_gaussian(&mut self) -> f32 {
        let u1 = self.next_uniform().max(f32::EPSILON);
        let u2 = self.next_uniform();
        let radius = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f32::consts::PI * u2;
        radius * theta.cos()
    }
}

