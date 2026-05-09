// SPDX-License-Identifier: Unlicense

//! Tests for `crate::browser::window::RenderLog` — converted from
//! `#[cfg(test)] mod tests {}` (Phase 3). The dedup contract for
//! the eprintln-in-render-hot-path fix from audit bug #10.

use crate::browser::window::RenderLog;

use super::{case, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("window::render_log_dedups_identical_messages",
             render_log_dedups_identical_messages),
        case("window::render_log_re_emits_when_message_changes",
             render_log_re_emits_when_message_changes),
        case("window::render_log_clears_on_ok_so_next_err_emits_again",
             render_log_clears_on_ok_so_next_err_emits_again),
        case("window::render_log_slots_are_independent",
             render_log_slots_are_independent),
    ]
}

fn render_log_dedups_identical_messages() -> Result<(), String> {
    // Pin: the same error string emitted twice updates the slot
    // once. The cached state and the stderr emission track 1:1,
    // so checking the cached state covers the dedup contract.
    let mut log = RenderLog::default();
    log.prepare_err("atlas oom");
    check_eq(log.last_prepare(), Some("text prepare: atlas oom"), "first emit")?;
    log.prepare_err("atlas oom");
    check_eq(log.last_prepare(), Some("text prepare: atlas oom"), "second is no-op")
}

fn render_log_re_emits_when_message_changes() -> Result<(), String> {
    let mut log = RenderLog::default();
    log.prepare_err("atlas oom");
    log.prepare_err("device lost");
    check_eq(log.last_prepare(), Some("text prepare: device lost"), "different msg overwrites")
}

fn render_log_clears_on_ok_so_next_err_emits_again() -> Result<(), String> {
    let mut log = RenderLog::default();
    log.prepare_err("atlas oom");
    log.prepare_ok();
    check_eq(log.last_prepare(), None, "ok clears slot")?;
    // After recovery + new failure, slot updates again rather
    // than being suppressed by the stale cache.
    log.prepare_err("atlas oom");
    check_eq(
        log.last_prepare(),
        Some("text prepare: atlas oom"),
        "post-recovery err re-emits",
    )
}

fn render_log_slots_are_independent() -> Result<(), String> {
    // prepare / render / frame each have their own slot; an
    // entry in one doesn't dedup against the others.
    let mut log = RenderLog::default();
    log.prepare_err("a");
    log.render_err("a");
    log.frame_err("a");
    check_eq(log.last_prepare(), Some("text prepare: a"), "prepare slot")?;
    check_eq(log.last_render(), Some("text render: a"), "render slot")?;
    check_eq(log.last_frame(), Some("render: a"), "frame slot")
}
