#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_ID="com.example.crosskit_example_android"

if [[ -z "${JAVA_HOME:-}" ]]; then
  if [[ -x "/Applications/Android Studio.app/Contents/jbr/Contents/Home/bin/java" ]]; then
    export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
  fi
fi

if ! command -v adb >/dev/null 2>&1; then
  echo "adb is required for Android example smoke tests" >&2
  exit 1
fi

if [[ -z "${ANDROID_SERIAL:-}" ]]; then
  online_devices="$(adb devices | awk 'NR > 1 && $2 == "device" { print $1 }')"
  if [[ -z "$online_devices" ]]; then
    echo "an online Android device or emulator is required" >&2
    exit 1
  fi

  selected_device=""
  while IFS= read -r device; do
    model="$(adb -s "$device" shell getprop ro.product.model 2>/dev/null | tr -d '\r')"
    if [[ "$model" != *ATD* ]]; then
      selected_device="$device"
      break
    fi
  done <<<"$online_devices"

  if [[ -z "$selected_device" ]]; then
    echo "no visible Android device found; ATD devices are rejected because screenshots can be black" >&2
    adb devices -l >&2
    exit 1
  fi

  export ANDROID_SERIAL="$selected_device"
fi

device_model="$(adb -s "$ANDROID_SERIAL" shell getprop ro.product.model 2>/dev/null | tr -d '\r')"
if [[ "$device_model" == *ATD* ]]; then
  echo "selected Android device is an ATD model ($device_model); use a visible emulator/device for screenshot gates" >&2
  exit 1
fi

echo "Using Android device: $ANDROID_SERIAL"

examples=(
  minimal-counter
  counter-list
  form-wizard
  search-refresh
  shopping-cart
  task-board
)

expected_text_for() {
  case "$1" in
    minimal-counter) echo "Minimal Counter" ;;
    counter-list) echo "Counter: 0" ;;
    form-wizard) echo "Form Wizard" ;;
    search-refresh) echo "Search Refresh" ;;
    shopping-cart) echo "Shopping Cart" ;;
    task-board) echo "Task Board" ;;
    *) echo "unknown example: $1" >&2; return 1 ;;
  esac
}

wait_for_app_installed() {
  local example="$1"
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    if adb shell pm path "$APP_ID" >/dev/null 2>&1 &&
      adb shell cmd package resolve-activity --brief "$APP_ID" | grep -Fq "$APP_ID/.MainActivity"; then
      return 0
    fi
    sleep 1
  done
  echo "$example: installed package or MainActivity was not visible to package manager" >&2
  adb shell pm list packages | grep -F "$APP_ID" >&2 || true
  adb shell cmd package resolve-activity --brief "$APP_ID" >&2 || true
  return 1
}

wait_for_ui_text() {
  local example="$1"
  local expected="$2"
  local output="$3"
  for _ in 1 2 3 4 5 6 7 8 9 10; do
    if adb shell uiautomator dump /sdcard/window.xml >/dev/null 2>&1 &&
      adb pull /sdcard/window.xml "$output" >/dev/null 2>&1; then
      if grep -Fq "Process system isn't responding" "$output"; then
        echo "$example: visible emulator is blocked by a system ANR dialog" >&2
        return 1
      fi
      if grep -Fq "$expected" "$output"; then
        return 0
      fi
    fi
    sleep 1
  done
  echo "$example: UI hierarchy did not contain expected text: $expected" >&2
  rg -o 'text="[^"]*"' "$output" >&2 || true
  return 1
}

run_connected_tests() {
  local example="$1"
  if (cd "$ROOT_DIR/examples/$example/android" && ./gradlew connectedDebugAndroidTest); then
    return 0
  fi

  echo "$example: connectedDebugAndroidTest failed once; retrying after adb reconnect wait" >&2
  adb wait-for-device
  sleep 3
  (cd "$ROOT_DIR/examples/$example/android" && ./gradlew connectedDebugAndroidTest)
}

tmp_dir="${TMPDIR:-/tmp}/cross-kit-android-smoke"
rm -rf "$tmp_dir"
mkdir -p "$tmp_dir"

validate_png_visible() {
  local png="$1"
  local example="$2"
  python3 - "$png" "$example" <<'PY'
import struct
import sys
import zlib

path, example = sys.argv[1], sys.argv[2]
data = open(path, "rb").read()
if not data.startswith(b"\x89PNG\r\n\x1a\n"):
    raise SystemExit(f"{example}: screenshot is not a PNG")

pos = 8
width = height = bit_depth = color_type = None
idat = bytearray()
while pos < len(data):
    length = struct.unpack(">I", data[pos:pos + 4])[0]
    chunk_type = data[pos + 4:pos + 8]
    chunk_data = data[pos + 8:pos + 8 + length]
    pos += 12 + length
    if chunk_type == b"IHDR":
        width, height, bit_depth, color_type, _, _, _ = struct.unpack(">IIBBBBB", chunk_data)
    elif chunk_type == b"IDAT":
        idat.extend(chunk_data)
    elif chunk_type == b"IEND":
        break

if width is None or height is None or bit_depth != 8 or color_type not in (2, 6):
    raise SystemExit(f"{example}: unsupported screenshot PNG format")

channels = 3 if color_type == 2 else 4
stride = width * channels
raw = zlib.decompress(bytes(idat))
rows = []
prev = [0] * stride
offset = 0

def paeth(a, b, c):
    p = a + b - c
    pa = abs(p - a)
    pb = abs(p - b)
    pc = abs(p - c)
    if pa <= pb and pa <= pc:
        return a
    if pb <= pc:
        return b
    return c

for _ in range(height):
    filter_type = raw[offset]
    offset += 1
    row = list(raw[offset:offset + stride])
    offset += stride
    for i in range(stride):
        left = row[i - channels] if i >= channels else 0
        up = prev[i]
        up_left = prev[i - channels] if i >= channels else 0
        if filter_type == 1:
            row[i] = (row[i] + left) & 0xFF
        elif filter_type == 2:
            row[i] = (row[i] + up) & 0xFF
        elif filter_type == 3:
            row[i] = (row[i] + ((left + up) // 2)) & 0xFF
        elif filter_type == 4:
            row[i] = (row[i] + paeth(left, up, up_left)) & 0xFF
        elif filter_type != 0:
            raise SystemExit(f"{example}: unsupported PNG filter {filter_type}")
    rows.append(row)
    prev = row

sample_step = max(1, (width * height) // 50000)
colors = set()
non_black = 0
non_white = 0
seen = 0
pixel_index = 0
for row in rows:
    for x in range(0, width * channels, channels):
        if pixel_index % sample_step == 0:
            rgb = tuple(row[x:x + 3])
            colors.add(rgb)
            if max(rgb) > 12:
                non_black += 1
            if min(rgb) < 243:
                non_white += 1
            seen += 1
        pixel_index += 1

if len(colors) < 8:
    raise SystemExit(f"{example}: screenshot has too few colors; likely blank or solid")
if non_black / max(seen, 1) < 0.05:
    raise SystemExit(f"{example}: screenshot is effectively black")
if non_white / max(seen, 1) < 0.005:
    raise SystemExit(f"{example}: screenshot is effectively empty white background")
PY
}

for example in "${examples[@]}"; do
  echo "== $example: package =="
  (cd "$ROOT_DIR" && cargo run -p cross-kit-cli -- android package --config "examples/$example/cross-kit.toml")

  echo "== $example: build and install =="
  adb uninstall "$APP_ID" >/dev/null 2>&1 || true
  (cd "$ROOT_DIR/examples/$example/android" && ./gradlew assembleDebug installDebug)
  wait_for_app_installed "$example"

  apk="$ROOT_DIR/examples/$example/android/app/build/outputs/apk/debug/app-debug.apk"
  apk_contents="$(unzip -l "$apk")"
  if ! grep -q "libjnidispatch.so" <<<"$apk_contents"; then
    echo "$example: APK is missing libjnidispatch.so from the JNA AAR" >&2
    exit 1
  fi
  if ! grep -q "libcross_kit_.*\\.so" <<<"$apk_contents"; then
    echo "$example: APK is missing the Cross-Kit native library" >&2
    exit 1
  fi

  echo "== $example: launch =="
  adb logcat -c >/dev/null 2>&1 || true
  adb shell am force-stop "$APP_ID"
  start_output="$(adb shell am start -W -n "$APP_ID/.MainActivity")"
  if ! grep -Fq "Status: ok" <<<"$start_output"; then
    echo "$example: am start failed" >&2
    echo "$start_output" >&2
    exit 1
  fi
  sleep 2

  if adb logcat -d | rg -i \
    "FATAL EXCEPTION|AndroidRuntime|UnsatisfiedLinkError|UnexpectedUniFFICallbackError|NullPointerException"; then
    echo "$example: launch produced a fatal Android log" >&2
    exit 1
  fi

  if ! adb shell dumpsys activity activities \
    | rg "topResumedActivity=.*${APP_ID}" >/dev/null; then
    echo "$example: app is not the top resumed activity after launch" >&2
    adb shell dumpsys activity activities | rg "topResumedActivity|mResumedActivity" >&2 || true
    exit 1
  fi

  wait_for_ui_text "$example" "$(expected_text_for "$example")" "$tmp_dir/$example-window.xml"

  adb exec-out screencap -p > "$tmp_dir/$example-screen.png"
  validate_png_visible "$tmp_dir/$example-screen.png" "$example"
  echo "$example: screenshot saved to $tmp_dir/$example-screen.png"

  echo "== $example: connected tests =="
  run_connected_tests "$example"
done

echo "Android examples are buildable, launchable, visible, and covered by instrumentation tests."
