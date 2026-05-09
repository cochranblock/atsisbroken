// SPDX-License-Identifier: Unlicense

//! Per-character human-like input. The browser dispatches real
//! keyboard events with Gaussian-jittered timing, mouse-focuses
//! before typing, and pauses between fields proportional to
//! field complexity. This is the core of the "looks human"
//! property.
//!
//! No `rand` dep — we use a small splitmix64 PRNG seeded once per
//! session. Deterministic for tests; sufficiently random in
//! production for the timing distribution we need.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_is_around_100wpm() {
        let p = HumanInputProfile::default();
        // 100 ms/char × 5 chars/word = 500 ms/word = 120 WPM.
        // Plus inter-field pauses on top. "Slightly faster than
        // real human" target.
        assert!(p.mean_char_delay_ms < 200.0);
        assert!(p.mean_char_delay_ms > 50.0);
    }

    #[test]
    fn fixed_distribution_returns_exact_mean() {
        let mut p = HumanInputProfile::default();
        p.timing_distribution = KeyTimingDistribution::Fixed;
        let mut t = InputTiming::new(p.clone());
        for _ in 0..10 {
            let d = t.next_char_delay();
            assert_eq!(d, Duration::from_secs_f32(p.mean_char_delay_ms / 1000.0));
        }
    }

    #[test]
    fn gaussian_distribution_centers_on_mean() {
        let p = HumanInputProfile {
            seed: 42,
            timing_distribution: KeyTimingDistribution::Gaussian,
            ..Default::default()
        };
        let mut t = InputTiming::new(p.clone());
        let n = 1000;
        let total_ms: f32 = (0..n)
            .map(|_| t.next_char_delay().as_secs_f32() * 1000.0)
            .sum();
        let avg = total_ms / n as f32;
        // Gaussian over 1000 samples should be within ~1 stddev / sqrt(n)
        // of the mean. With stddev=30 and n=1000 that's ~1ms tolerance;
        // we use 5ms for ample slack.
        assert!(
            (avg - p.mean_char_delay_ms).abs() < 5.0,
            "mean drift: avg={avg}, target={}",
            p.mean_char_delay_ms
        );
    }

    #[test]
    fn uniform_distribution_stays_in_range() {
        let p = HumanInputProfile {
            seed: 7,
            timing_distribution: KeyTimingDistribution::Uniform,
            mean_char_delay_ms: 100.0,
            char_delay_stddev_ms: 20.0,
            ..Default::default()
        };
        let mut t = InputTiming::new(p);
        for _ in 0..200 {
            let ms = t.next_char_delay().as_secs_f32() * 1000.0;
            // Uniform [mean-stddev, mean+stddev] = [80, 120].
            // Floor clamp to 1ms doesn't bite at these means.
            assert!(ms >= 80.0 - 0.01 && ms <= 120.0 + 0.01,
                "uniform out of range: {ms}");
        }
    }

    #[test]
    fn timing_is_seeded_deterministically() {
        let p = HumanInputProfile {
            seed: 12345,
            timing_distribution: KeyTimingDistribution::Gaussian,
            ..Default::default()
        };
        let mut a = InputTiming::new(p.clone());
        let mut b = InputTiming::new(p);
        for _ in 0..20 {
            assert_eq!(a.next_char_delay(), b.next_char_delay());
        }
    }

    #[test]
    fn delay_is_never_zero() {
        // Hard contract: even with Gaussian tails dipping below
        // zero, we floor at 1 ms. A zero-delay character defeats
        // the human-like property.
        let p = HumanInputProfile {
            seed: 999,
            mean_char_delay_ms: 5.0,
            char_delay_stddev_ms: 50.0,
            timing_distribution: KeyTimingDistribution::Gaussian,
            ..Default::default()
        };
        let mut t = InputTiming::new(p);
        for _ in 0..500 {
            let d = t.next_char_delay();
            assert!(d.as_micros() >= 1000, "zero or sub-ms delay sampled");
        }
    }

    #[test]
    fn key_dwell_is_realistic() {
        // 60 ms is in the realistic range for keyboards (real
        // measurements: 40-120 ms). Pinned so accidental drift
        // surfaces.
        let p = HumanInputProfile::default();
        let t = InputTiming::new(p.clone());
        let dwell_ms = t.key_dwell().as_secs_f32() * 1000.0;
        assert!(dwell_ms >= 30.0 && dwell_ms <= 120.0);
    }
}
