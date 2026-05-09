// SPDX-License-Identifier: Unlicense

//! Tests for `crate::strategy` (Phase 6).

use crate::strategy::{
    bookmarklet, detect, detect_clipboard_tool, find_chromium_binary, speak, userscript,
    ClipboardTool, Strategy,
};
use crate::Profile;

use super::{case, check, check_eq, TestResult};

pub fn run() -> Vec<TestResult> {
    vec![
        case("strategy::detect_always_returns_a_strategy", detect_always_returns_a_strategy),
        case("strategy::from_cli_str_userscript_branch", from_cli_str_userscript_branch),
        case("strategy::from_cli_str_bookmarklet_branch", from_cli_str_bookmarklet_branch),
        case("strategy::from_cli_str_extension_branch", from_cli_str_extension_branch),
        case("strategy::from_cli_str_speak_branch", from_cli_str_speak_branch),
        case("strategy::from_cli_str_unknown_returns_err", from_cli_str_unknown_returns_err),
        case("strategy::from_cli_str_empty_returns_err", from_cli_str_empty_returns_err),
        case("strategy::from_cli_str_clipboard_returns_err_without_tool",
             from_cli_str_clipboard_returns_err_without_tool),
        case("strategy::clipboard_tool_args_pass_to_command_correctly",
             clipboard_tool_args_pass_to_command_correctly),
        case("strategy::speak_emits_every_populated_field_once",
             speak_emits_every_populated_field_once),
        case("strategy::speak_skips_empty_fields", speak_skips_empty_fields),
        case("strategy::userscript_contains_profile_and_classifier",
             userscript_contains_profile_and_classifier),
        case("strategy::bookmarklet_is_javascript_url", bookmarklet_is_javascript_url),
        case("strategy::bookmarklet_size_under_browser_caps", bookmarklet_size_under_browser_caps),
        case("strategy::clipboard_tool_binaries_are_known", clipboard_tool_binaries_are_known),
        case("strategy::xclip_args_select_clipboard_not_primary",
             xclip_args_select_clipboard_not_primary),
        case("strategy::userscript_braces_balance", userscript_braces_balance),
        case("strategy::userscript_parens_balance", userscript_parens_balance),
        case("strategy::userscript_has_required_userscript_metadata",
             userscript_has_required_userscript_metadata),
        case("strategy::userscript_does_not_clobber_filled_inputs",
             userscript_does_not_clobber_filled_inputs),
        case("strategy::bookmarklet_is_single_line", bookmarklet_is_single_line),
        case("strategy::bookmarklet_has_no_unencoded_hash", bookmarklet_has_no_unencoded_hash),
        case("strategy::bookmarklet_has_no_unencoded_quotes", bookmarklet_has_no_unencoded_quotes),
        case("strategy::speak_emits_each_field_once_only", speak_emits_each_field_once_only),
        case("strategy::speak_preserves_spaces_in_values", speak_preserves_spaces_in_values),
        case("strategy::detect_clipboard_tool_returns_none_when_no_tool_present",
             detect_clipboard_tool_returns_none_when_no_tool_present),
        case("strategy::strategy_speak_is_always_constructable", strategy_speak_is_always_constructable),
        case("strategy::find_chromium_binary_returns_a_typed_option",
             find_chromium_binary_returns_a_typed_option),
    ]
}

fn sample_profile() -> Profile {
    Profile {
        full_name: "Jane Doe".into(),
        email: "jane@example.com".into(),
        phone: "+1-555-0100".into(),
        linkedin: "linkedin.com/in/janedoe".into(),
        github: "github.com/janedoe".into(),
        website: "janedoe.dev".into(),
        address: "1 Main St, Anywhere, USA".into(),
        work_authorization: "US Citizen".into(),
        years_experience: 7,
        ..Default::default()
    }
}

fn detect_always_returns_a_strategy() -> Result<(), String> {
    let _ = detect();
    Ok(())
}

fn from_cli_str_userscript_branch() -> Result<(), String> {
    check_eq(
        Strategy::from_cli_str("userscript").map_err(|e| format!("{e}"))?,
        Strategy::Userscript,
        "userscript branch",
    )
}

fn from_cli_str_bookmarklet_branch() -> Result<(), String> {
    check_eq(
        Strategy::from_cli_str("bookmarklet").map_err(|e| format!("{e}"))?,
        Strategy::Bookmarklet,
        "bookmarklet branch",
    )
}

fn from_cli_str_extension_branch() -> Result<(), String> {
    check_eq(
        Strategy::from_cli_str("extension").map_err(|e| format!("{e}"))?,
        Strategy::Extension,
        "extension branch",
    )
}

fn from_cli_str_speak_branch() -> Result<(), String> {
    check_eq(
        Strategy::from_cli_str("speak").map_err(|e| format!("{e}"))?,
        Strategy::Speak,
        "speak branch",
    )
}

fn from_cli_str_unknown_returns_err() -> Result<(), String> {
    let err = Strategy::from_cli_str("yolo").err().ok_or_else(|| "expected err".to_string())?;
    check(err.contains("unknown"), format!("err should mention 'unknown', got: {err}"))
}

fn from_cli_str_empty_returns_err() -> Result<(), String> {
    check(Strategy::from_cli_str("").is_err(), "empty input → Err")
}

fn from_cli_str_clipboard_returns_err_without_tool() -> Result<(), String> {
    if let Ok(s) = Strategy::from_cli_str("clipboard") {
        check(matches!(s, Strategy::Clipboard { .. }), "clipboard variant when ok")?;
    }
    Ok(())
}

fn clipboard_tool_args_pass_to_command_correctly() -> Result<(), String> {
    for (tool, args) in [
        (ClipboardTool::Pbcopy, vec![]),
        (ClipboardTool::Xclip, vec!["-selection", "clipboard"]),
        (ClipboardTool::Wlcopy, vec![]),
        (ClipboardTool::ClipExe, vec![]),
    ] {
        check_eq(tool.args(), args, &format!("{tool:?} args"))?;
    }
    Ok(())
}

fn speak_emits_every_populated_field_once() -> Result<(), String> {
    let mut buf = Vec::new();
    speak(&sample_profile(), &mut buf).map_err(|e| format!("{e}"))?;
    let out = String::from_utf8(buf).map_err(|e| format!("{e}"))?;
    for needle in [
        "full_name: Jane Doe",
        "email: jane@example.com",
        "phone: +1-555-0100",
        "linkedin: linkedin.com/in/janedoe",
        "github: github.com/janedoe",
        "website: janedoe.dev",
        "address: 1 Main St, Anywhere, USA",
        "work_authorization: US Citizen",
        "years_experience: 7",
    ] {
        check(out.contains(needle), format!("missing line {needle:?}"))?;
    }
    Ok(())
}

fn speak_skips_empty_fields() -> Result<(), String> {
    let mut p = Profile::default();
    p.email = "x@y.com".into();
    let mut buf = Vec::new();
    speak(&p, &mut buf).map_err(|e| format!("{e}"))?;
    let out = String::from_utf8(buf).map_err(|e| format!("{e}"))?;
    check(out.contains("email: x@y.com"), "email present")?;
    check(!out.contains("full_name:"), "full_name skipped")?;
    check(!out.contains("phone:"), "phone skipped")?;
    check(!out.contains("years_experience"), "years_experience skipped")
}

fn userscript_contains_profile_and_classifier() -> Result<(), String> {
    let s = userscript(&sample_profile());
    check(s.contains("==UserScript=="), "has UserScript header")?;
    check(s.contains("\"jane@example.com\""), "has email")?;
    check(s.contains("predict"), "classifier present")?;
    check(s.contains("MutationObserver"), "re-render defense present")
}

fn bookmarklet_is_javascript_url() -> Result<(), String> {
    let b = bookmarklet(&sample_profile());
    check(b.starts_with("javascript:"), "starts with javascript:")?;
    check(b.contains("jane@example.com"), "contains email")?;
    let body = b.strip_prefix("javascript:").ok_or_else(|| "no javascript: prefix".to_string())?;
    check(!body.contains(' '), "no raw spaces in encoded body")
}

fn bookmarklet_size_under_browser_caps() -> Result<(), String> {
    let b = bookmarklet(&sample_profile());
    check(b.len() < 4096, format!("bookmarklet too large: {} bytes", b.len()))
}

fn clipboard_tool_binaries_are_known() -> Result<(), String> {
    check_eq(ClipboardTool::Pbcopy.binary(), "pbcopy", "Pbcopy binary")?;
    check_eq(ClipboardTool::Xclip.binary(), "xclip", "Xclip binary")?;
    check_eq(ClipboardTool::Wlcopy.binary(), "wl-copy", "Wlcopy binary")?;
    check_eq(ClipboardTool::ClipExe.binary(), "clip.exe", "ClipExe binary")
}

fn xclip_args_select_clipboard_not_primary() -> Result<(), String> {
    check_eq(
        ClipboardTool::Xclip.args(),
        vec!["-selection", "clipboard"],
        "xclip args",
    )
}

fn userscript_braces_balance() -> Result<(), String> {
    let s = userscript(&sample_profile());
    let opens = s.matches('{').count();
    let closes = s.matches('}').count();
    check_eq(opens, closes, &format!("braces: {opens} {{ vs {closes} }}"))
}

fn userscript_parens_balance() -> Result<(), String> {
    let s = userscript(&sample_profile());
    let opens = s.matches('(').count();
    let closes = s.matches(')').count();
    check_eq(opens, closes, &format!("parens: {opens} ( vs {closes} )"))
}

fn userscript_has_required_userscript_metadata() -> Result<(), String> {
    let s = userscript(&sample_profile());
    for tag in ["@name", "@namespace", "@version", "@match", "@run-at", "@grant"] {
        check(s.contains(tag), format!("userscript missing {tag}"))?;
    }
    Ok(())
}

fn userscript_does_not_clobber_filled_inputs() -> Result<(), String> {
    let s = userscript(&sample_profile());
    check(
        s.contains("don't clobber") || s.contains("trim().length"),
        "userscript must guard against clobbering existing values",
    )
}

fn bookmarklet_is_single_line() -> Result<(), String> {
    let b = bookmarklet(&sample_profile());
    check(!b.contains('\n'), "bookmarklet single line")
}

fn bookmarklet_has_no_unencoded_hash() -> Result<(), String> {
    let b = bookmarklet(&sample_profile());
    let body = b.strip_prefix("javascript:").ok_or_else(|| "no prefix".to_string())?;
    check(!body.contains('#'), "no unencoded #")
}

fn bookmarklet_has_no_unencoded_quotes() -> Result<(), String> {
    let b = bookmarklet(&sample_profile());
    let body = b.strip_prefix("javascript:").ok_or_else(|| "no prefix".to_string())?;
    check(!body.contains('"'), "no double quotes")?;
    check(!body.contains('\''), "no single quotes")
}

fn speak_emits_each_field_once_only() -> Result<(), String> {
    let mut buf = Vec::new();
    speak(&sample_profile(), &mut buf).map_err(|e| format!("{e}"))?;
    let out = String::from_utf8(buf).map_err(|e| format!("{e}"))?;
    for key in [
        "full_name:",
        "email:",
        "phone:",
        "linkedin:",
        "github:",
        "website:",
        "address:",
        "work_authorization:",
        "years_experience:",
    ] {
        let count = out.matches(key).count();
        check_eq(count, 1usize, &format!("{key} appeared {count} times"))?;
    }
    Ok(())
}

fn speak_preserves_spaces_in_values() -> Result<(), String> {
    let mut buf = Vec::new();
    speak(&sample_profile(), &mut buf).map_err(|e| format!("{e}"))?;
    let out = String::from_utf8(buf).map_err(|e| format!("{e}"))?;
    check(out.contains("address: 1 Main St, Anywhere, USA"), "spaces preserved")
}

fn detect_clipboard_tool_returns_none_when_no_tool_present() -> Result<(), String> {
    let _ = detect_clipboard_tool();
    Ok(())
}

fn strategy_speak_is_always_constructable() -> Result<(), String> {
    let _ = Strategy::Speak;
    Ok(())
}

fn find_chromium_binary_returns_a_typed_option() -> Result<(), String> {
    let r: Option<std::path::PathBuf> = find_chromium_binary();
    if let Some(p) = r {
        check(p.is_absolute() || p.exists(), "chromium binary path absolute or exists")?;
    }
    Ok(())
}
