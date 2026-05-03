// SPDX-License-Identifier: Unlicense
// atsisbroken-android — JNI surface. Wraps the parent atsisbroken crate's
// schemas + (eventually) classifier for the Android WebView shell.

use atsisbroken::{seed_corpus_fingerprint, version, FieldDescriptor, Mode};
use serde::Serialize;

#[derive(Serialize)]
struct Status<'a> {
    version: &'a str,
    seed_corpus_fingerprint: u32,
    default_mode: Mode,
}

/// Pure-Rust core fns that the JNI layer calls. Kept separate so they're
/// testable without a JVM and so we can lift them into other transports
/// (Tauri, iOS, etc.) without rewriting.
pub fn status_json() -> String {
    serde_json::to_string(&Status {
        version: version(),
        seed_corpus_fingerprint: seed_corpus_fingerprint(),
        default_mode: Mode::default(),
    })
    .unwrap_or_else(|_| "{}".into())
}

pub fn classify_json(field_json: &str) -> Result<String, serde_json::Error> {
    let _f: FieldDescriptor = serde_json::from_str(field_json)?;
    // Classifier hookup lands when the trainer + on-disk model are wired.
    // Until then, return the explicit "unknown" sentinel so the WebView
    // glue knows not to autofill.
    Ok(serde_json::json!({"key": "unknown"}).to_string())
}

#[cfg(target_os = "android")]
mod jni_layer {
    use super::*;
    use jni::objects::{JClass, JString};
    use jni::sys::jstring;
    use jni::JNIEnv;

    fn rust_to_jstring<'a>(env: &mut JNIEnv<'a>, s: &str) -> jstring {
        env.new_string(s)
            .map(|js| js.into_raw())
            .unwrap_or(std::ptr::null_mut())
    }

    #[no_mangle]
    pub extern "system" fn Java_org_cochranblock_atsisbroken_Native_nativeStatus<'a>(
        mut env: JNIEnv<'a>,
        _class: JClass<'a>,
    ) -> jstring {
        rust_to_jstring(&mut env, &status_json())
    }

    #[no_mangle]
    pub extern "system" fn Java_org_cochranblock_atsisbroken_Native_nativeClassify<'a>(
        mut env: JNIEnv<'a>,
        _class: JClass<'a>,
        field_json: JString<'a>,
    ) -> jstring {
        let s: String = match env.get_string(&field_json) {
            Ok(js) => js.into(),
            Err(_) => return rust_to_jstring(&mut env, r#"{"key":"unknown"}"#),
        };
        let out = classify_json(&s).unwrap_or_else(|_| r#"{"key":"unknown"}"#.into());
        rust_to_jstring(&mut env, &out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_json_is_valid() {
        let s = status_json();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert!(v.get("version").is_some());
        assert!(v.get("seed_corpus_fingerprint").is_some());
        assert!(v.get("default_mode").is_some());
    }

    #[test]
    fn classify_unknown_field_returns_unknown() {
        let f = r#"{"label":"Salary expectation","placeholder":"","aria_label":"","name":"salary","id":"","kind":"number"}"#;
        let out = classify_json(f).unwrap();
        assert!(out.contains("unknown"));
    }

    #[test]
    fn status_json_carries_version_string() {
        let v: serde_json::Value = serde_json::from_str(&status_json()).unwrap();
        assert_eq!(v["version"].as_str().unwrap(), atsisbroken::version());
    }

    #[test]
    fn status_json_default_mode_is_training_wheels() {
        let v: serde_json::Value = serde_json::from_str(&status_json()).unwrap();
        assert_eq!(v["default_mode"].as_str().unwrap(), "training_wheels");
    }

    #[test]
    fn status_json_fingerprint_matches_core() {
        let v: serde_json::Value = serde_json::from_str(&status_json()).unwrap();
        assert_eq!(
            v["seed_corpus_fingerprint"].as_u64().unwrap() as u32,
            atsisbroken::seed_corpus_fingerprint()
        );
    }

    #[test]
    fn classify_malformed_json_errors() {
        let out = classify_json("{not json");
        assert!(out.is_err());
    }

    #[test]
    fn classify_returns_well_formed_json() {
        let f = r#"{"label":"Email","placeholder":"","aria_label":"","name":"email","id":"","kind":"email"}"#;
        let out = classify_json(f).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!(v["key"].is_string());
    }
}
