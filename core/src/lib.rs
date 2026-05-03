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
