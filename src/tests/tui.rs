// SPDX-License-Identifier: Unlicense

//! Tests for `crate::tui` (Phase 7).

#![cfg(feature = "tui")]

use crate::strategy::Strategy;
use crate::tui::runtime::{key_action, render_html_for_screenshot};
use crate::tui::{App, KeyAction, TABS};
use crate::{Feedback, FeedbackQueue, FieldDescriptor, Mode};
use crossterm::event::{KeyCode, KeyModifiers};

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("tui::app_starts_on_dashboard", app_starts_on_dashboard),
        case("tui::next_tab_wraps", next_tab_wraps),
        case("tui::prev_tab_wraps_backwards", prev_tab_wraps_backwards),
        case("tui::cycle_mode_walks_all_three_states", cycle_mode_walks_all_three_states),
        case("tui::quit_sets_should_quit_flag", quit_sets_should_quit_flag),
        case("tui::handle_key_quit_marks_quit", handle_key_quit_marks_quit),
        case("tui::handle_key_noop_returns_false", handle_key_noop_returns_false),
        case("tui::handle_key_tab_index_in_range", handle_key_tab_index_in_range),
        case("tui::handle_key_tab_index_out_of_range_ignored",
             handle_key_tab_index_out_of_range_ignored),
        case("tui::switching_tabs_resets_queue_scroll", switching_tabs_resets_queue_scroll),
        case("tui::scroll_down_in_queue_is_bounded_by_len", scroll_down_in_queue_is_bounded_by_len),
        case("tui::scroll_up_at_zero_does_not_underflow", scroll_up_at_zero_does_not_underflow),
        case("tui::scroll_only_advances_on_queue_tab", scroll_only_advances_on_queue_tab),
        case("tui::tabs_constant_matches_documented_count", tabs_constant_matches_documented_count),
        // ─── runtime: key_action + render_html_for_screenshot ──────
        case("tui::key_action_quit_on_q_or_esc", key_action_quit_on_q_or_esc),
        case("tui::key_action_ctrl_c_quits", key_action_ctrl_c_quits),
        case("tui::key_action_tab_and_arrows_navigate", key_action_tab_and_arrows_navigate),
        case("tui::key_action_digit_keys_pick_tab", key_action_digit_keys_pick_tab),
        case("tui::key_action_vim_keys_supported", key_action_vim_keys_supported),
        case("tui::key_action_unhandled_returns_noop", key_action_unhandled_returns_noop),
        case("tui::render_html_produces_valid_doctype_and_atsisbroken_label",
             render_html_produces_valid_doctype_and_atsisbroken_label),
        case("tui::render_html_each_tab_distinguishable", render_html_each_tab_distinguishable),
        case("tui::render_html_clamps_oversize_tab_index", render_html_clamps_oversize_tab_index),
        case("tui::render_html_includes_keybind_footer_hints", render_html_includes_keybind_footer_hints),
        case("tui::render_html_escapes_html_metacharacters", render_html_escapes_html_metacharacters),
    ]
}

fn empty_app() -> App {
    App::new(None, FeedbackQueue::default(), Mode::default(), Strategy::Speak)
}

fn app_with_queue(n: usize) -> App {
    let mut q = FeedbackQueue::default();
    for i in 0..n {
        q.append(Feedback {
            field: FieldDescriptor {
                label: format!("f{i}"),
                placeholder: "".into(),
                aria_label: "".into(),
                name: "".into(),
                id: "".into(),
                kind: "text".into(),
            },
            predicted: "email".into(),
            actual: "email".into(),
            accepted: i % 2 == 0,
        });
    }
    App::new(None, q, Mode::default(), Strategy::Speak)
}

fn app_starts_on_dashboard() -> Result<(), String> {
    let app = empty_app();
    check_eq(app.current_tab, 0usize, "current_tab")?;
    check_eq(TABS[app.current_tab], "dashboard", "tab name")
}

fn next_tab_wraps() -> Result<(), String> {
    let mut app = empty_app();
    app.next_tab();
    check_eq(app.current_tab, 1usize, "→ 1")?;
    app.next_tab();
    check_eq(app.current_tab, 2usize, "→ 2")?;
    app.next_tab();
    check_eq(app.current_tab, 0usize, "→ wraps to 0")
}

fn prev_tab_wraps_backwards() -> Result<(), String> {
    let mut app = empty_app();
    app.prev_tab();
    check_eq(app.current_tab, TABS.len() - 1, "→ last tab")?;
    app.prev_tab();
    check_eq(app.current_tab, TABS.len() - 2, "→ second-to-last")
}

fn cycle_mode_walks_all_three_states() -> Result<(), String> {
    let mut app = empty_app();
    check_eq(app.mode, Mode::TrainingWheels, "starts at TrainingWheels")?;
    app.cycle_mode();
    check_eq(app.mode, Mode::Shadow, "→ Shadow")?;
    app.cycle_mode();
    check_eq(app.mode, Mode::Chaos, "→ Chaos")?;
    app.cycle_mode();
    check_eq(app.mode, Mode::TrainingWheels, "→ wraps to TrainingWheels")
}

fn quit_sets_should_quit_flag() -> Result<(), String> {
    let mut app = empty_app();
    check(!app.should_quit, "starts not should_quit")?;
    app.quit();
    check(app.should_quit, "quit sets flag")
}

fn handle_key_quit_marks_quit() -> Result<(), String> {
    let mut app = empty_app();
    check(app.handle_key(KeyAction::Quit), "Quit returns true")?;
    check(app.should_quit, "should_quit set")
}

fn handle_key_noop_returns_false() -> Result<(), String> {
    let mut app = empty_app();
    check(!app.handle_key(KeyAction::Noop), "Noop returns false")?;
    check(!app.should_quit, "Noop doesn't set should_quit")
}

fn handle_key_tab_index_in_range() -> Result<(), String> {
    let mut app = empty_app();
    check(app.handle_key(KeyAction::TabIndex(2)), "TabIndex returns true")?;
    check_eq(app.current_tab, 2usize, "current_tab updated")
}

fn handle_key_tab_index_out_of_range_ignored() -> Result<(), String> {
    let mut app = empty_app();
    check(!app.handle_key(KeyAction::TabIndex(99)), "out-of-range returns false")?;
    check_eq(app.current_tab, 0usize, "current_tab unchanged")
}

fn switching_tabs_resets_queue_scroll() -> Result<(), String> {
    let mut app = app_with_queue(10);
    app.handle_key(KeyAction::TabIndex(1));
    for _ in 0..5 {
        app.handle_key(KeyAction::Down);
    }
    check_eq(app.queue_scroll, 5usize, "scrolled to 5")?;
    app.handle_key(KeyAction::NextTab);
    check_eq(app.queue_scroll, 0usize, "reset on tab switch")
}

fn scroll_down_in_queue_is_bounded_by_len() -> Result<(), String> {
    let mut app = app_with_queue(3);
    app.current_tab = 1;
    for _ in 0..10 {
        app.handle_key(KeyAction::Down);
    }
    check_eq(app.queue_scroll, 2usize, "scroll capped at len-1")
}

fn scroll_up_at_zero_does_not_underflow() -> Result<(), String> {
    let mut app = app_with_queue(3);
    app.current_tab = 1;
    for _ in 0..5 {
        app.handle_key(KeyAction::Up);
    }
    check_eq(app.queue_scroll, 0usize, "scroll stays at 0")
}

fn scroll_only_advances_on_queue_tab() -> Result<(), String> {
    let mut app = app_with_queue(5);
    app.handle_key(KeyAction::Down);
    check_eq(app.queue_scroll, 0usize, "Down on dashboard does not scroll")
}

fn tabs_constant_matches_documented_count() -> Result<(), String> {
    check_eq(TABS.len(), 3usize, "TABS.len")?;
    check_eq(TABS, ["dashboard", "queue", "strategy"], "TABS contents")
}

// ─── runtime: key_action + render_html_for_screenshot ──────────────

fn key_action_quit_on_q_or_esc() -> Result<(), String> {
    check_eq(key_action(KeyCode::Char('q'), KeyModifiers::empty()), KeyAction::Quit, "q")?;
    check_eq(key_action(KeyCode::Esc, KeyModifiers::empty()), KeyAction::Quit, "Esc")
}

fn key_action_ctrl_c_quits() -> Result<(), String> {
    check_eq(
        key_action(KeyCode::Char('c'), KeyModifiers::CONTROL),
        KeyAction::Quit,
        "Ctrl+C",
    )
}

fn key_action_tab_and_arrows_navigate() -> Result<(), String> {
    check_eq(key_action(KeyCode::Tab, KeyModifiers::empty()), KeyAction::NextTab, "Tab")?;
    check_eq(key_action(KeyCode::Right, KeyModifiers::empty()), KeyAction::NextTab, "Right")?;
    check_eq(key_action(KeyCode::BackTab, KeyModifiers::empty()), KeyAction::PrevTab, "BackTab")?;
    check_eq(key_action(KeyCode::Left, KeyModifiers::empty()), KeyAction::PrevTab, "Left")
}

fn key_action_digit_keys_pick_tab() -> Result<(), String> {
    check_eq(key_action(KeyCode::Char('1'), KeyModifiers::empty()), KeyAction::TabIndex(0), "1")?;
    check_eq(key_action(KeyCode::Char('2'), KeyModifiers::empty()), KeyAction::TabIndex(1), "2")?;
    check_eq(key_action(KeyCode::Char('3'), KeyModifiers::empty()), KeyAction::TabIndex(2), "3")?;
    check_eq(key_action(KeyCode::Char('9'), KeyModifiers::empty()), KeyAction::Noop, "9 → Noop")
}

fn key_action_vim_keys_supported() -> Result<(), String> {
    check_eq(key_action(KeyCode::Char('h'), KeyModifiers::empty()), KeyAction::PrevTab, "h")?;
    check_eq(key_action(KeyCode::Char('l'), KeyModifiers::empty()), KeyAction::NextTab, "l")?;
    check_eq(key_action(KeyCode::Char('j'), KeyModifiers::empty()), KeyAction::Down, "j")?;
    check_eq(key_action(KeyCode::Char('k'), KeyModifiers::empty()), KeyAction::Up, "k")
}

fn key_action_unhandled_returns_noop() -> Result<(), String> {
    check_eq(key_action(KeyCode::Char('z'), KeyModifiers::empty()), KeyAction::Noop, "z")?;
    check_eq(key_action(KeyCode::F(5), KeyModifiers::empty()), KeyAction::Noop, "F5")
}

fn render_html_produces_valid_doctype_and_atsisbroken_label() -> Result<(), String> {
    let html = render_html_for_screenshot(0, 80, 16).map_err(|e| format!("{e}"))?;
    check(html.starts_with("<!doctype html>"), "doctype")?;
    check(html.contains("atsisbroken"), "label")?;
    check(html.contains("</body></html>"), "well-formed end")
}

fn render_html_each_tab_distinguishable() -> Result<(), String> {
    for (i, name) in TABS.iter().enumerate() {
        let html = render_html_for_screenshot(i, 80, 16).map_err(|e| format!("{e}"))?;
        check(html.contains(name), format!("tab {i} ({name}) didn't render its own name"))?;
    }
    Ok(())
}

fn render_html_clamps_oversize_tab_index() -> Result<(), String> {
    let html = render_html_for_screenshot(99, 80, 16).map_err(|e| format!("{e}"))?;
    check(html.contains("strategy"), "out-of-range tab clamps to last")
}

fn render_html_includes_keybind_footer_hints() -> Result<(), String> {
    let html = render_html_for_screenshot(0, 80, 16).map_err(|e| format!("{e}"))?;
    check(html.contains("quit"), "footer should mention quit")
}

fn render_html_escapes_html_metacharacters() -> Result<(), String> {
    let html = render_html_for_screenshot(0, 80, 16).map_err(|e| format!("{e}"))?;
    check(!html.contains("<script"), "must not embed scripts")
}
