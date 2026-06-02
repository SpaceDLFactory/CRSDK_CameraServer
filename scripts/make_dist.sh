#!/usr/bin/env bash
# 바이너리 배포본(dist/) 조립 — Sony SDK 라이선스가 허용하는 "앱 바이너리 + SDK 라이브러리"
# 형태로 패키징한다. 라이브러리 자체 재배포가 아니라 앱에 동봉하는 형태.
#
# 사용: ./scripts/make_dist.sh   (프로젝트 루트에서)
# 결과: dist/  (gitignore 대상 — Sony dylib 포함)
set -euo pipefail
cd "$(dirname "$0")/.."

SDK_LIB="CrSDK_v2.01.00_20260203a_Mac/RemoteCli/external/crsdk"
DIST="dist"

[ -d "$SDK_LIB" ] || { echo "✗ SDK 없음: $SDK_LIB (Sony에서 받아 배치)"; exit 1; }

echo "▶ release 빌드"
export DYLD_LIBRARY_PATH="${DYLD_LIBRARY_PATH:-}:$(pwd)/$SDK_LIB"
cargo build --release -p crsdk_server

echo "▶ dist 조립"
rm -rf "$DIST"
mkdir -p "$DIST/Contents/Frameworks"

cp target/release/crsdk_server "$DIST/"
cp -R crsdk_server/web "$DIST/web"

# libCr_Core + 모니터 프로토콜 라이브러리 (실행파일 옆 → DYLD_LIBRARY_PATH로 탐색)
cp "$SDK_LIB"/libCr_Core.dylib "$SDK_LIB"/libmonitor_protocol*.dylib "$DIST/"
# CrAdapter (libCr_Core가 <exe>/Contents/Frameworks/CrAdapter 에서 로드)
cp -R "$SDK_LIB/CrAdapter" "$DIST/Contents/Frameworks/CrAdapter"

# 실행 런처: 자기 폴더 기준 DYLD 경로 설정 후 실행
cat > "$DIST/run.command" <<'LAUNCH'
#!/usr/bin/env bash
cd "$(dirname "$0")"
export DYLD_LIBRARY_PATH="$PWD:${DYLD_LIBRARY_PATH:-}"
# macOS ptpcamerad가 USB 카메라 접근을 방해 → 서버 내장 억제기가 처리(권한 필요시 sudo)
exec ./crsdk_server
LAUNCH
chmod +x "$DIST/run.command"

# .DS_Store 정리
find "$DIST" -name .DS_Store -delete 2>/dev/null || true

echo "✓ 완료: $DIST/ ($(du -sh "$DIST" | cut -f1))"
echo "  실행: open $DIST/run.command  또는  (cd $DIST && ./run.command)"
echo "  UI:   http://localhost:8080/web/index.html"
