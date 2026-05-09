// SPDX-License-Identifier: Unlicense

//! Tests for `crate::browser::input` — converted from
//! `#[cfg(test)] mod tests {}` to the cochranblock exopack
//! pattern (Phase 2).

use std::time::Duration;

use crate::browser::input::InputTiming;
use crate::browser::{HumanInputProfile, KeyTimingDistribution};

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("input::default_profile_is_around_100wpm", default_profile_is_around_100wpm),
        case("input::fixed_distribution_returns_exact_mean", fixed_distribution_returns_exact_mean),
        case("input::gaussian_distribution_centers_on_mean", gaussian_distribution_centers_on_mean),
        case("input::uniform_distribution_stays_in_range", uniform_distribution_stays_in_range),
        case("input::timing_is_seeded_deterministically", timing_is_seeded_deterministically),
        case("input::delay_is_never_zero", delay_is_never_zero),
        case("input::key_dwell_is_realistic", key_dwell_is_realistic),
    ]
}

fn default_profile_is_around_100wpm() -> Result<(), String> {
    let p = HumanInputProfile::default();
    // 100 ms/char × 5 chars/word = 500 ms/word = 120 WPM.
    // Plus inter-field pauses on top.
    check(
        p.mean_char_delay_ms < 200.0,
        format!("mean_char_delay_ms={} should be < 200", p.mean_char_delay_ms),
    )?;
    check(
        p.mean_char_delay_ms > 50.0,
        format!("mean_char_delay_ms={} should be > 50", p.mean_char_delay_ms),
    )
}

fn fixed_distribution_returns_exact_mean() -> Result<(), String> {
    let mut p = HumanInputProfile::default();
    p.timing_distribution = KeyTimingDistribution::Fixed;
    let expected = Duration::from_secs_f32(p.mean_char_delay_ms / 1000.0);
    let mut t = InputTiming::new(p);
    for _ in 0..10 {
        let d = t.next_char_delay();
        check_eq(d, expected, "fixed distribution sample")?;
    }
    Ok(())
}

fn gaussian_distribution_centers_on_mean() -> Result<(), String> {
    let p = HumanInputProfile {
        seed: 42,
        timing_distribution: KeyTimingDistribution::Gaussian,
        ..Default::default()
    };
    let target_mean = p.mean_char_delay_ms;
    let mut t = InputTiming::new(p);
    let n = 1000;
    let total_ms: f32 = (0..n)
        .map(|_| t.next_char_delay().as_secs_f32() * 1000.0)
        .sum();
    let avg = total_ms / n as f32;
    // Gaussian over 1000 samples should be within ~1 stddev/sqrt(n)
    // of the mean. With stddev=30 and n=1000 that's ~1ms tolerance;
    // we use 5ms for ample slack.
    check(
        (avg - target_mean).abs() < 5.0,
        format!("mean drift: avg={avg}, target={target_mean}"),
    )
}

fn uniform_distribution_stays_in_range() -> Result<(), String> {
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
        check(
            ms >= 80.0 - 0.01 && ms <= 120.0 + 0.01,
            format!("uniform out of range: {ms}"),
        )?;
    }
    Ok(())
}

fn timing_is_seeded_deterministically() -> Result<(), String> {
    let p = HumanInputProfile {
        seed: 12345,
        timing_distribution: KeyTimingDistribution::Gaussian,
        ..Default::default()
    };
    let mut a = InputTiming::new(p.clone());
    let mut b = InputTiming::new(p);
    for _ in 0..20 {
        check_eq(a.next_char_delay(), b.next_char_delay(), "seeded determinism")?;
    }
    Ok(())
}

fn delay_is_never_zero() -> Result<(), String> {
    // Even with Gaussian tails dipping below zero, we floor at 1 ms.
    // A zero-delay character defeats the human-like property.
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
        check(
            d.as_micros() >= 1000,
            format!("zero or sub-ms delay sampled: {}us", d.as_micros()),
        )?;
    }
    Ok(())
}

fn key_dwell_is_realistic() -> Result<(), String> {
    // 60 ms is in the realistic range for keyboards (real
    // measurements: 40-120 ms). Pinned so accidental drift surfaces.
    let p = HumanInputProfile::default();
    let t = InputTiming::new(p);
    let dwell_ms = t.key_dwell().as_secs_f32() * 1000.0;
    check(
        dwell_ms >= 30.0 && dwell_ms <= 120.0,
        format!("key_dwell {dwell_ms} ms outside realistic range"),
    )
}
