#!/usr/bin/env bash
# 앱 아이콘 생성 — 첫 작품(달 사진)에서 macOS 스타일 둥근 아이콘(.icns)을 만든다.
# gallery/first-moon.jpg 중앙(=달)을 정사각형으로 크롭 → swift로 둥근 모서리 렌더 → iconset → icns.
# 사용: ./scripts/make_icon.sh   결과: assets/AppIcon.icns
set -euo pipefail
cd "$(dirname "$0")/.."

SRC="gallery/first-moon.jpg"
OUT="assets/AppIcon.icns"
[ -f "$SRC" ] || { echo "✗ 소스 없음: $SRC"; exit 1; }
mkdir -p assets
TMP="$(mktemp -d)"

echo "▶ 달 중앙 정사각형 크롭"
sips -c 520 520 "$SRC" --out "$TMP/sq.png" >/dev/null   # 이미지 중심 ≈ 달 중심

echo "▶ 둥근 모서리 1024 렌더 (swift/CoreGraphics)"
cat > "$TMP/icon.swift" <<'SWIFT'
import AppKit
let args = CommandLine.arguments
guard let src = NSImage(contentsOfFile: args[1]) else { exit(1) }
let S: CGFloat = 1024
let rep = NSBitmapImageRep(bitmapDataPlanes: nil, pixelsWide: Int(S), pixelsHigh: Int(S),
  bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
  colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!
let ctx = NSGraphicsContext(bitmapImageRep: rep)!
NSGraphicsContext.current = ctx
let g = ctx.cgContext
g.clear(CGRect(x: 0, y: 0, width: S, height: S))
let margin = S * 0.085                       // 투명 여백(Apple 아이콘 그리드 근사)
let rect = CGRect(x: margin, y: margin, width: S - 2*margin, height: S - 2*margin)
let radius = rect.width * 0.2237             // 연속 모서리(squircle) 근사 반경
g.beginPath()
g.addPath(CGPath(roundedRect: rect, cornerWidth: radius, cornerHeight: radius, transform: nil))
g.clip()
g.setFillColor(NSColor.black.cgColor)
g.fill(rect)
let sw = src.size.width, sh = src.size.height
let scale = max(rect.width / sw, rect.height / sh)   // AspectFill
let dw = sw * scale, dh = sh * scale
src.draw(in: CGRect(x: rect.midX - dw/2, y: rect.midY - dh/2, width: dw, height: dh),
         from: .zero, operation: .copy, fraction: 1.0)
NSGraphicsContext.current = nil
guard let png = rep.representation(using: .png, properties: [:]) else { exit(1) }
try! png.write(to: URL(fileURLWithPath: args[2]))
SWIFT
swift "$TMP/icon.swift" "$TMP/sq.png" "$TMP/icon1024.png"

echo "▶ iconset → icns"
ISET="$TMP/AppIcon.iconset"; mkdir -p "$ISET"
for s in 16 32 128 256 512; do
  sips -z "$s" "$s"       "$TMP/icon1024.png" --out "$ISET/icon_${s}x${s}.png"    >/dev/null
  sips -z "$((s*2))" "$((s*2))" "$TMP/icon1024.png" --out "$ISET/icon_${s}x${s}@2x.png" >/dev/null
done
iconutil -c icns "$ISET" -o "$OUT"
rm -rf "$TMP"
echo "✓ $OUT ($(du -h "$OUT" | cut -f1))"
