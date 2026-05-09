// SPDX-License-Identifier: Unlicense
//! Resume → `Profile` parser.
//!
//! Heuristic, hand-rolled, no regex dep. The user's pasted resume text
//! is scanned for: email, phone, LinkedIn URL, GitHub URL, generic
//! website URL, and a plausible full name (first non-empty line that
//! contains no contact tokens). The full text is preserved as
//! `raw_resume_text` for downstream free-text generation.
//!
//! This is deliberately *low* sophistication — the trained classifier is
//! the answer for ambiguous cases. The parser only has to nail the
//! obvious 90% so the user's first form-fill works.

use crate::Profile;

pub fn parse_resume(text: &str) -> Profile {
    let mut p = Profile {
        raw_resume_text: text.to_string(),
        ..Default::default()
    };

    if let Some(email) = find_email(text) {
        p.email = email;
    }
    if let Some(phone) = find_phone(text) {
        p.phone = phone;
    }
    if let Some(li) = find_url_containing(text, "linkedin.com/in/") {
        p.linkedin = li;
    }
    if let Some(gh) = find_url_containing(text, "github.com/") {
        p.github = gh;
    }
    if let Some(site) = find_other_url(text, &p.linkedin, &p.github) {
        p.website = site;
    }
    if let Some(name) = find_name(text, &p.email, &p.phone) {
        // Split on whitespace: last token = last_name, everything
        // before = first_name. "Jane Q. Doe" → first="Jane Q.",
        // last="Doe". One-word names ("Jane") set full_name only —
        // first/last stay empty (avoids dropping the user's whole
        // name into a "Last name" field).
        let parts: Vec<&str> = name.split_whitespace().collect();
        p.full_name = name.clone();
        if let [head @ .., last] = parts.as_slice() {
            if !head.is_empty() {
                p.last_name = last.to_string();
                p.first_name = head.join(" ");
            }
        }
    }

    p
}

fn find_email(text: &str) -> Option<String> {
    // Find '@', then walk left/right within email-character class.
    let bytes = text.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b != b'@' {
            continue;
        }
        let start = (0..i)
            .rev()
            .take_while(|&j| is_email_char(bytes[j]))
            .last()
            .unwrap_or(i);
        let end = (i + 1..bytes.len())
            .take_while(|&j| is_email_char(bytes[j]))
            .last()
            .map(|j| j + 1)
            .unwrap_or(i + 1);
        if start < i && end > i + 1 {
            let candidate = &text[start..end];
            // Domain part must contain a dot.
            if candidate[(i - start + 1)..].contains('.') {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

fn is_email_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'+' | b'%')
}

fn find_phone(text: &str) -> Option<String> {
    // Scan for runs of digits (with optional dashes / spaces / parens / +)
    // of length 10–15 digits when stripped.
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if !is_phone_start(c) {
            i += 1;
            continue;
        }
        let start = i;
        let mut end = i;
        let mut digits = 0;
        while end < bytes.len() && is_phone_char(bytes[end]) {
            if bytes[end].is_ascii_digit() {
                digits += 1;
            }
            end += 1;
        }
        if (10..=15).contains(&digits) {
            return Some(text[start..end].trim().to_string());
        }
        i = end.max(start + 1);
    }
    None
}

fn is_phone_start(b: u8) -> bool {
    b == b'+' || b == b'(' || b.is_ascii_digit()
}

fn is_phone_char(b: u8) -> bool {
    b.is_ascii_digit() || matches!(b, b' ' | b'-' | b'(' | b')' | b'.' | b'+')
}

fn find_url_containing(text: &str, needle: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let pos = lower.find(needle)?;
    // Walk left to start-of-URL (whitespace boundary), right to end.
    let bytes = text.as_bytes();
    let start = (0..pos)
        .rev()
        .take_while(|&j| !bytes[j].is_ascii_whitespace())
        .last()
        .unwrap_or(pos);
    let end = (pos..bytes.len())
        .take_while(|&j| !bytes[j].is_ascii_whitespace())
        .last()
        .map(|j| j + 1)
        .unwrap_or(bytes.len());
    Some(text[start..end].trim_end_matches(|c: char| ".,;:".contains(c)).to_string())
}

fn find_other_url(text: &str, exclude_linkedin: &str, exclude_github: &str) -> Option<String> {
    // Look for anything that looks like a URL but isn't one of the two we
    // already extracted. Cheap detector: starts with "http" or contains
    // ".com" / ".dev" / ".io" / ".me".
    for line in text.lines() {
        for tok in line.split_whitespace() {
            let t = tok.trim_end_matches(|c: char| ".,;:)".contains(c));
            let looks_url = t.starts_with("http://")
                || t.starts_with("https://")
                || (t.contains('.')
                    && (t.ends_with(".com")
                        || t.ends_with(".dev")
                        || t.ends_with(".io")
                        || t.ends_with(".me")
                        || t.ends_with(".org")
                        || t.ends_with(".net")));
            if !looks_url {
                continue;
            }
            if t == exclude_linkedin || t == exclude_github {
                continue;
            }
            if t.contains('@') {
                continue; // it's an email, not a URL
            }
            // Skip the linkedin/github canonical hosts even if URLs differ in scheme.
            if t.contains("linkedin.com") || t.contains("github.com") {
                continue;
            }
            return Some(t.to_string());
        }
    }
    None
}

fn find_name(text: &str, email: &str, phone: &str) -> Option<String> {
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.contains('@')
            || line.contains("http")
            || (!email.is_empty() && line.contains(email))
            || (!phone.is_empty() && line.contains(phone))
        {
            continue;
        }
        // First plausible name-like line: 2–5 words, mostly letters.
        let words: Vec<&str> = line.split_whitespace().collect();
        if !(2..=5).contains(&words.len()) {
            continue;
        }
        let mostly_letters = words
            .iter()
            .all(|w| w.chars().filter(|c| c.is_alphabetic()).count() * 2 >= w.len());
        if mostly_letters {
            return Some(line.to_string());
        }
    }
    None
}

