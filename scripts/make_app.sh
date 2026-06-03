#!/usr/bin/env bash
# macOS .app 번들 패키징 — Sony SDK 라이브러리를 앱 번들 안(Contents/Frameworks)에 동봉.
# 단일 단위(.app)로 묶어 라이선스의 "inseparable" 형태에 더 부합하고, 더블클릭 실행됨.
#
# 사용: ./scripts/make_app.sh
# 결과: dist/A7C Tether.app  (gitignore — Sony dylib 포함)
set -euo pipefail
cd "$(dirname "$0")/.."

SDK_LIB="CrSDK_v2.01.00_20260203a_Mac/RemoteCli/external/crsdk"
APP="dist/A7C Tether.app"
BIN="crsdk_server"
VERSION="$(grep '^version' crsdk_server/Cargo.toml | head -1 | cut -d'"' -f2)"  # Cargo에서 버전 일원화

[ -d "$SDK_LIB" ] || { echo "✗ SDK 없음: $SDK_LIB"; exit 1; }

echo "▶ release 빌드"
export DYLD_LIBRARY_PATH="${DYLD_LIBRARY_PATH:-}:$(pwd)/$SDK_LIB"
cargo build --release -p crsdk_server

echo "▶ .app 번들 조립"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Frameworks" "$APP/Contents/Resources"

# 실행파일
cp "target/release/$BIN" "$APP/Contents/MacOS/$BIN"
# UI (Resources/web → web_dir()가 ../Resources/web 로 탐색)
cp -R crsdk_server/web "$APP/Contents/Resources/web"
# SDK 라이브러리 (Frameworks)
cp "$SDK_LIB"/libCr_Core.dylib "$SDK_LIB"/libmonitor_protocol*.dylib "$APP/Contents/Frameworks/"
cp -R "$SDK_LIB/CrAdapter" "$APP/Contents/Frameworks/CrAdapter"   # libCr_Core가 bundlePath/Contents/Frameworks/CrAdapter 에서 로드

# 바이너리가 @rpath/libCr_Core.dylib 를 Contents/Frameworks 에서 찾도록 rpath 추가
install_name_tool -add_rpath "@executable_path/../Frameworks" "$APP/Contents/MacOS/$BIN" 2>/dev/null || true

# Info.plist
cat > "$APP/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>            <string>A7C Tether</string>
  <key>CFBundleDisplayName</key>     <string>A7C Tether</string>
  <key>CFBundleIdentifier</key>      <string>film.neko.a7ctether</string>
  <key>CFBundleVersion</key>         <string>$VERSION</string>
  <key>CFBundleShortVersionString</key> <string>$VERSION</string>
  <key>CFBundlePackageType</key>     <string>APPL</string>
  <key>CFBundleExecutable</key>      <string>$BIN</string>
  <key>LSMinimumSystemVersion</key>  <string>12.0</string>
  <key>NSHighResolutionCapable</key> <true/>
</dict>
</plist>
PLIST

# ad-hoc 재서명 (Apple Silicon 실행 요건). 번들 내 Sony dylib이 미서명이라 실행파일만
# 서명하면 실패 → dylib/플러그인 먼저, 그다음 실행파일, 마지막에 번들 전체 순으로 서명.
echo "▶ ad-hoc 코드서명"
find "$APP/Contents/Frameworks" -name "*.dylib" -exec codesign --force --sign - {} \; 2>/dev/null || true
codesign --force --sign - "$APP/Contents/MacOS/$BIN" 2>/dev/null || true
codesign --force --sign - "$APP" 2>/dev/null \
  && codesign --verify "$APP" 2>/dev/null && echo "  서명 검증 OK" \
  || echo "  (codesign 실패 — 실행 시 우클릭→열기 또는 quarantine 해제 필요)"

find "$APP" -name .DS_Store -delete 2>/dev/null || true

echo "✓ 완료: $APP ($(du -sh "$APP" | cut -f1))"
echo "  실행: open \"$APP\"   (처음엔 우클릭→열기 또는: xattr -dr com.apple.quarantine \"$APP\")"
echo "  UI:   http://localhost:8080/web/index.html"
