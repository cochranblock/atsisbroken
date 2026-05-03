#!/usr/bin/env bash
# Capture CLI subcommand output as PNG screenshots via headless Chromium.
# Output: docs/screenshots/*.png
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$ROOT/docs/screenshots"
BIN="$ROOT/target/debug/atsisbroken"
CHROMIUM="${CHROMIUM:-chromium}"

mkdir -p "$OUT"

render() {
    local name="$1"
    local title="$2"
    local body="$3"
    local html
    html=$(cat <<HTML
<!DOCTYPE html><html><head><meta charset="utf-8"><style>
body{margin:0;background:#0e1117;color:#c9d1d9;font:14px/1.5 'JetBrains Mono','Menlo',monospace;padding:24px}
.bar{display:flex;align-items:center;gap:8px;padding:0 0 12px 0;border-bottom:1px solid #30363d;margin-bottom:16px}
.dot{width:12px;height:12px;border-radius:50%}
.r{background:#ff5f56}.y{background:#ffbd2e}.g{background:#27c93f}
.t{margin-left:12px;color:#8b949e;font-size:12px}
pre{margin:0;white-space:pre-wrap;color:#c9d1d9}
.cmd{color:#7ee787}.dim{color:#8b949e}
</style></head><body>
<div class="bar"><span class="dot r"></span><span class="dot y"></span><span class="dot g"></span><span class="t">$title</span></div>
<pre><span class="cmd">\$ </span>$body</pre>
</body></html>
HTML
)
    local b64
    b64=$(printf '%s' "$html" | base64 -w0)
    "$CHROMIUM" --headless --disable-gpu --no-sandbox --hide-scrollbars \
        --screenshot="$OUT/$name.png" --window-size=960,540 \
        "data:text/html;base64,$b64" >/dev/null 2>&1
    echo "  rendered: $OUT/$name.png"
}

shot() {
    local name="$1"; shift
    local title="$1"; shift
    local cmd_label="atsisbroken"
    # display just `atsisbroken <args>` not the full path
    for arg in "${@:2}"; do cmd_label+=" $arg"; done
    local body
    body=$("$@" 2>&1 | sed 's/&/\&amp;/g; s/</\&lt;/g; s/>/\&gt;/g')
    render "$name" "$title" "${cmd_label}
${body}"
}

echo "Capturing screenshots →"
shot 01-help     "atsisbroken --help"          "$BIN" --help
shot 02-status   "atsisbroken status"           "$BIN" status
shot 03-init     "atsisbroken init"             "$BIN" init
shot 04-run      "atsisbroken run"              "$BIN" run
shot 05-graduate "atsisbroken graduate"         "$BIN" graduate
shot 06-sync     "atsisbroken sync"             "$BIN" sync

echo "Done. ls $OUT:"
ls -la "$OUT"
