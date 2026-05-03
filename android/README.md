# atsisbroken — Android

Native Android app. Same architecture as the desktop binary, different
transport: instead of CDP to a separate Chromium, the Android `WebView`
hosts the browsing surface and the Rust core is loaded via JNI.

## Layout

```
android/
├── Cargo.toml              # cdylib JNI crate (atsisbroken-android)
├── src/lib.rs              # JNI exports — Rust → Kotlin bridge
├── app/                    # Android Studio app module
│   ├── build.gradle.kts
│   └── src/main/
│       ├── AndroidManifest.xml
│       ├── java/org/cochranblock/atsisbroken/
│       │   └── MainActivity.kt
│       └── res/
│           ├── layout/activity_main.xml
│           └── values/strings.xml
└── README.md
```

## Build

The Rust JNI lib is cross-compiled per ABI with `cargo-ndk`:

```sh
cargo install cargo-ndk
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android

cd android
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 \
    -o app/src/main/jniLibs build --profile=diamond-edge
```

Then open `android/app/` in Android Studio (or use `./gradlew assembleRelease`).

## Architecture (mobile)

```
[user] → [WebView (Android)] ← JS bridge → [Kotlin glue] ← JNI → [Rust core]
              loads URL                                            classifier
              extracts fields                                      feedback queue
              fills inputs                                         profile loader
```

The Rust core (`src/lib.rs` here) wraps `atsisbroken` (the desktop crate)
and exposes a JNI surface:

- `nativeInit(resumeText: String): String` — parse resume, seed classifier
- `nativeClassify(fieldJson: String): String` — predict key for one field
- `nativeRecordFeedback(feedbackJson: String)` — append to queue
- `nativeStatus(): String` — JSON snapshot for the status screen

Same `Profile`, same `FieldDescriptor`, same `Mode`, same `Feedback` —
the schemas are shared with the desktop binary by depending on the
parent crate via `path = ".."`.

## Play Store posture

- Permissions requested: `INTERNET` (only because WebView needs it to
  load the user's chosen URL — no calls go anywhere else).
- No `READ_EXTERNAL_STORAGE`, no `RECORD_AUDIO`, no location.
- Privacy policy: identical to desktop — local-first, no cloud, no
  accounts. The Play listing should link to `LICENSE-PROVENANCE.md`
  and `docs/USER_STORIES.md` for diligence.
- Free, no IAP, no ads. Ever.
