// SPDX-License-Identifier: Unlicense

//! Tests for `crate::browser::text` — converted from
//! `#[cfg(test)] mod tests {}` (Phase 3).

use crate::browser::text::{page_runs, TextFamily};

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("text::page_runs_produce_three_blocks", page_runs_produce_three_blocks),
        case("text::page_runs_url_above_title_above_body", page_runs_url_above_title_above_body),
        case("text::page_runs_clip_within_surface", page_runs_clip_within_surface),
        case("text::empty_strings_still_produce_runs", empty_strings_still_produce_runs),
        case("text::small_window_doesnt_underflow_body_height",
             small_window_doesnt_underflow_body_height),
    ]
}

fn page_runs_produce_three_blocks() -> Result<(), String> {
    let runs = page_runs(
        "atsisbroken://home",
        "atsisbroken",
        "Connect what you've made.",
        1280,
        800,
    );
    check_eq(runs.len(), 3, "runs count")?;
    check_eq(runs[0].text.clone(), "atsisbroken://home".to_string(), "url run text")?;
    check_eq(runs[0].family, TextFamily::Monospace, "url run family")?;
    check_eq(runs[1].text.clone(), "atsisbroken".to_string(), "title run text")?;
    check_eq(
        runs[2].text.clone(),
        "Connect what you've made.".to_string(),
        "body run text",
    )
}

fn page_runs_url_above_title_above_body() -> Result<(), String> {
    let runs = page_runs("u", "t", "b", 1000, 600);
    check(runs[0].y < runs[1].y, "URL must be above title")?;
    check(runs[1].y < runs[2].y, "title must be above body")
}

fn page_runs_clip_within_surface() -> Result<(), String> {
    let runs = page_runs("u", "t", "b", 800, 600);
    for r in &runs {
        check(r.x + r.width as f32 <= 800.0, format!("run x+width exceeds surface: {}", r.x + r.width as f32))?;
        check(r.y + r.height as f32 <= 600.0, format!("run y+height exceeds surface: {}", r.y + r.height as f32))?;
    }
    Ok(())
}

fn empty_strings_still_produce_runs() -> Result<(), String> {
    // Glyphon handles empty buffers; we shouldn't suppress empty
    // text either — the shell may legitimately want to hide a
    // section by passing empty.
    let runs = page_runs("", "", "", 1280, 800);
    check_eq(runs.len(), 3, "runs count for empty strings")?;
    for r in &runs {
        check(r.text.is_empty(), "run text should be empty")?;
    }
    Ok(())
}

fn small_window_doesnt_underflow_body_height() -> Result<(), String> {
    // If the user resizes the window very small, body height could
    // underflow. Saturating math protects.
    let runs = page_runs("u", "t", "b", 200, 80);
    check(runs[2].height as i64 >= 0, "body height must not underflow")
}
