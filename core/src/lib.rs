// SPDX-License-Identifier: Unlicense
// atsisbroken-core — resume model, profile, field-matching. Compiles to WASM and to native rlib.

use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Profile {
    pub full_name: String,
    pub email: String,
    pub phone: String,
    pub address: String,
    pub linkedin: String,
    pub github: String,
    pub website: String,
    pub work_authorization: String,
    pub years_experience: u8,
    pub experience: Vec<Experience>,
    pub education: Vec<Education>,
    pub skills: Vec<String>,
    pub raw_resume_text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Experience {
    pub company: String,
    pub title: String,
    pub start: String,
    pub end: String,
    pub bullets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Education {
    pub school: String,
    pub degree: String,
    pub field: String,
    pub start: String,
    pub end: String,
    pub gpa: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDescriptor {
    pub label: String,
    pub placeholder: String,
    pub aria_label: String,
    pub name: String,
    pub id: String,
    pub kind: String,
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Map a form field to a Profile key. Pure keyword heuristic for the scaffold —
/// the real implementation will run the custom field-deduction model loaded
/// from a bundled .safetensors via kova-engine inference. Empty string = unknown.
pub fn predict_field_key(f: &FieldDescriptor) -> &'static str {
    let hay = format!(
        "{} {} {} {} {}",
        f.label, f.placeholder, f.aria_label, f.name, f.id
    )
    .to_lowercase();
    let has = |needle: &str| hay.contains(needle);

    if has("email") {
        "email"
    } else if has("phone") || has("mobile") || has("tel") {
        "phone"
    } else if has("linkedin") {
        "linkedin"
    } else if has("github") {
        "github"
    } else if has("website") || has("portfolio") || has("url") {
        "website"
    } else if has("address") || has("street") || has("city") || has("zip") {
        "address"
    } else if has("authoriz") || has("visa") || has("sponsor") || has("work_auth") {
        "work_authorization"
    } else if (has("year") || has("yrs")) && has("exp") {
        "years_experience"
    } else if has("first") && has("name") {
        "full_name"
    } else if has("last") && has("name") {
        "full_name"
    } else if has("full") && has("name") {
        "full_name"
    } else if has("name") {
        "full_name"
    } else {
        ""
    }
}

// ─── WASM exports (Chrome extension boundary) ──────────────────────────────
// content.js calls these from the extension. Native builds skip the bindings.

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn version_js() -> String {
    version().to_string()
}

/// Take a JS object describing a form field, return the predicted Profile key.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn predict_field_key_js(field: JsValue) -> Result<String, JsValue> {
    let f: FieldDescriptor =
        serde_wasm_bindgen::from_value(field).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(predict_field_key(&f).to_string())
}

// ─── tests ─────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    fn fd(label: &str, placeholder: &str, aria: &str, name: &str, id: &str) -> FieldDescriptor {
        FieldDescriptor {
            label: label.into(),
            placeholder: placeholder.into(),
            aria_label: aria.into(),
            name: name.into(),
            id: id.into(),
            kind: "text".into(),
        }
    }

    /// Canonical fixture set. Exopack TRIPLE SIMS gate hashes this
    /// after running predict_field_key on every input. Output must be
    /// byte-identical across three independent runs.
    fn fixtures() -> Vec<(FieldDescriptor, &'static str)> {
        vec![
            (fd("Email", "", "", "email", "email"), "email"),
            (fd("", "your.email@example.com", "", "", ""), "email"),
            (fd("", "", "Mobile phone number", "", ""), "phone"),
            (fd("", "", "", "user_phone_mobile", ""), "phone"),
            (fd("LinkedIn URL", "", "", "linkedin_url", ""), "linkedin"),
            (fd("GitHub", "", "", "", ""), "github"),
            (fd("Personal website", "", "", "", ""), "website"),
            (fd("Street address", "", "", "address1", ""), "address"),
            (fd("Are you authorized to work?", "", "", "", ""), "work_authorization"),
            (fd("Visa sponsorship required?", "", "", "", ""), "work_authorization"),
            (fd("Years of experience", "", "", "yrs_exp", ""), "years_experience"),
            (fd("First name", "", "", "fname", ""), "full_name"),
            (fd("Last name", "", "", "lname", ""), "full_name"),
            (fd("Full name", "", "", "name", ""), "full_name"),
            (fd("Salary expectation", "", "", "", ""), ""), // unknown → empty
            (fd("Why do you want this job?", "", "", "", ""), ""), // free-text → empty
        ]
    }

    #[test]
    fn predict_field_key_matches_fixtures() {
        for (f, expected) in fixtures() {
            assert_eq!(
                predict_field_key(&f),
                expected,
                "label={:?} placeholder={:?} name={:?}",
                f.label,
                f.placeholder,
                f.name
            );
        }
    }

    /// TRIPLE SIMS in-test: run the same fixture set three times and assert the
    /// resulting key sequence is byte-identical each pass. This is the
    /// determinism contract the exopack gate enforces externally.
    #[test]
    fn triple_sims_determinism() {
        let run = || -> Vec<String> {
            fixtures()
                .into_iter()
                .map(|(f, _)| predict_field_key(&f).to_string())
                .collect()
        };
        let s1 = run();
        let s2 = run();
        let s3 = run();
        assert_eq!(s1, s2, "sim 1 vs sim 2 diverged");
        assert_eq!(s2, s3, "sim 2 vs sim 3 diverged");
    }

    #[test]
    fn no_fabrication_for_unknown_fields() {
        // Critical user-trust property (P6 / A6): unknown fields return empty,
        // never a guess. Extension treats "" as "do not autofill".
        let unknown = fd("How many siblings do you have?", "", "", "", "");
        assert_eq!(predict_field_key(&unknown), "");
    }
}
