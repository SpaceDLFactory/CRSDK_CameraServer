# scripts/package-win.ps1 — TetherMoon Windows 배포 zip 생성
#
# 사용 (repo 루트에서, 관리자 불필요):
#   set "LIBCLANG_PATH=F:\LLVM\bin"
#   powershell -ExecutionPolicy Bypass -File scripts\package-win.ps1
#
# 산출물: dist\TetherMoon-win-x64\ (폴더형) + dist\TetherMoon-win-x64.zip
# 레이아웃: exe + Cr_Core.dll/monitor_protocol*.dll + CrAdapter\ + web\ + driver\ + README.txt
# (web_dir()가 exe 옆 web\ 를 1순위로 찾고, Cr_Core는 exe 옆, 플러그인은 exe 옆 CrAdapter\ 에서 로드)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not $env:LIBCLANG_PATH) { Write-Warning "LIBCLANG_PATH 미설정 — bindgen이 실패할 수 있음" }

# 1) 릴리스 빌드 (build.rs가 DLL을 target\release 옆에 복사)
cargo build --release -p crsdk_server
if ($LASTEXITCODE -ne 0) { throw "cargo build 실패" }

# 2) 스테이징
$stage = Join-Path $root "dist\TetherMoon-win-x64"
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force -Path $stage | Out-Null

$rel = Join-Path $root "target\release"
Copy-Item (Join-Path $rel "crsdk_server.exe") $stage
Get-ChildItem $rel -Filter *.dll | Copy-Item -Destination $stage
Copy-Item (Join-Path $rel "CrAdapter") $stage -Recurse
Copy-Item (Join-Path $root "crsdk_server\web") (Join-Path $stage "web") -Recurse

# 3) libusbK 드라이버 동봉 (winusb_driver\ 가 있으면 — Sony SDK의 Driver.zip 압축해제본)
$drv = Join-Path $root "winusb_driver"
if (Test-Path $drv) {
    Copy-Item $drv (Join-Path $stage "driver") -Recurse
} else {
    Write-Warning "winusb_driver\ 없음 — 드라이버는 패키지에서 제외됨(README 안내만 포함)"
}

# 4) README.txt (최종 사용자용)
$readme = @"
TetherMoon (Windows x64)
========================

[최초 1회: libusbK 드라이버 설치]
  1. 카메라를 USB로 연결하고, 카메라 메뉴에서 USB 원격(PC Remote)을 켠다.
  2. 장치관리자 → '휴대용 장치'/'카메라' 아래의 ILCE-... 우클릭 → 드라이버 업데이트
  3. '내 컴퓨터에서 찾아보기' → '직접 선택' → [디스크 있음] → driver\srcameradriver.inf
  4. 게시자 경고가 뜨면 '이 드라이버 설치'를 누른다.
  5. 장치가 'libusbK USB Devices / Sony Remote Control Camera' 로 바뀌면 성공.
  (driver\ 폴더가 없으면 Sony Camera Remote SDK의 Driver.zip 에서 받는다.)

[실행]
  crsdk_server.exe 더블클릭 → 기본 브라우저가 http://localhost:8080/web/index.html 로 열림.
  같은 Wi-Fi의 폰에서는 콘솔 창에 표시되는 http://<PC-IP>:8080/web/index.html 로 접속.

[종료]
  웹 UI의 종료 버튼, 또는 콘솔 창을 닫는다. (강제 종료 시 카메라를 USB 재연결해야 할 수 있음)

[문제 해결]
  - 'no cameras detected': 드라이버 미설치 또는 카메라 PC Remote 미설정.
  - 재연결이 ConnectTimeout: 카메라 USB를 뺐다 다시 연결.
"@
Set-Content -Encoding UTF8 -Path (Join-Path $stage "README.txt") -Value $readme

# 5) zip
$zip = Join-Path $root "dist\TetherMoon-win-x64.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path "$stage\*" -DestinationPath $zip
Write-Output ("packaged: " + $zip)
Write-Output ("staged  : " + $stage)
