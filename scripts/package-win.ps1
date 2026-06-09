# scripts/package-win.ps1 — TetherMoon Windows 배포 zip 생성
#
# 사용 (repo 루트에서, 관리자 불필요):
#   set "LIBCLANG_PATH=F:\LLVM\bin"
#   powershell -ExecutionPolicy Bypass -File scripts\package-win.ps1            # 전체(로컬/인스톨러용)
#   powershell -ExecutionPolicy Bypass -File scripts\package-win.ps1 -NoSony    # 공개배포용(Sony 바이너리 제외)
#
# 전체:   dist\TetherMoon-win-x64\           exe + Cr_Core/monitor_protocol + CrAdapter\ + VC런타임 + web\ + driver\ + README
# -NoSony: dist\TetherMoon-win-x64-portable\  exe + VC런타임 + web\ + README (Sony SDK DLL/CrAdapter/driver 제외 — 사용자 배치)
#   (Sony SDK·libusbK 드라이버는 재배포 라이선스 이슈로 공개 배포에서 제외. exe는 우리 코드, VC런타임은 MS 재배포 허용.)

param([switch]$NoSony)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if (-not $env:LIBCLANG_PATH) { Write-Warning "LIBCLANG_PATH 미설정 — bindgen이 실패할 수 있음" }

# 0) 실행 중인 인스턴스 정리 (dist\ DLL을 잡고 있으면 스테이징 삭제가 거부됨)
if (Get-Process crsdk_server -ErrorAction SilentlyContinue) {
    try { Invoke-WebRequest -UseBasicParsing -TimeoutSec 5 -Method Post "http://localhost:8080/api/quit" | Out-Null } catch {}
    Start-Sleep 2
    Get-Process crsdk_server -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep 1
}

# 1) 릴리스 빌드 (build.rs가 SDK DLL을 target\release 옆에 복사)
cargo build --release -p crsdk_server
if ($LASTEXITCODE -ne 0) { throw "cargo build 실패" }

# 2) 스테이징
$name  = if ($NoSony) { "TetherMoon-win-x64-portable" } else { "TetherMoon-win-x64" }
$stage = Join-Path $root ("dist\" + $name)
if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
New-Item -ItemType Directory -Force -Path $stage | Out-Null

$rel = Join-Path $root "target\release"
Copy-Item (Join-Path $rel "crsdk_server.exe") $stage
Copy-Item (Join-Path $root "crsdk_server\web") (Join-Path $stage "web") -Recurse

# 2b) VC++ 런타임 DLL을 exe 옆에 동봉 (app-local). exe·Cr_Core 모두 의존하며 Windows 기본 미포함이라
#     클린 머신에서 'VCRUNTIME140.dll 없음'으로 실행 실패. MS가 app-local 재배포 허용(vswhere로 탐색).
$crtNeeded = @("msvcp140.dll","vcruntime140.dll","vcruntime140_1.dll")
$crtDir = $null
$cands = @()
if ($env:VCToolsRedistDir) { $cands += (Join-Path $env:VCToolsRedistDir "x64\Microsoft.VC*.CRT") }
$vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path $vswhere) {
    $vs = & $vswhere -products * -latest -property installationPath 2>$null
    if ($vs) { $cands += (Join-Path $vs "VC\Redist\MSVC\*\x64\Microsoft.VC*.CRT") }
}
foreach ($c in $cands) {
    $d = Get-ChildItem $c -Directory -ErrorAction SilentlyContinue |
         Where-Object { Test-Path (Join-Path $_.FullName "vcruntime140.dll") } |
         Select-Object -First 1
    if ($d) { $crtDir = $d.FullName; break }
}
if ($crtDir) {
    foreach ($d in $crtNeeded) { Copy-Item (Join-Path $crtDir $d) $stage -Force }
    Write-Output ("VC++ runtime: " + $crtDir)
} else {
    Write-Warning "VC++ 런타임 DLL을 못 찾음 — 대상 PC에 VC++ 2015-2022 재배포(x64) 필요"
}

if (-not $NoSony) {
    # 3a) Sony SDK DLL (Cr_Core, monitor_protocol*) — build.rs가 target\release 옆에 복사해 둔 것
    Get-ChildItem $rel -Filter *.dll | Copy-Item -Destination $stage
    Copy-Item (Join-Path $rel "CrAdapter") $stage -Recurse

    # 3b) libusbK 드라이버 동봉 (winusb_driver\ 가 있으면 — Sony SDK의 Driver.zip 압축해제본)
    $drv = Join-Path $root "winusb_driver"
    if (Test-Path $drv) {
        Copy-Item $drv (Join-Path $stage "driver") -Recurse
        # 인스톨러가 TrustedPublisher 에 등록하도록 Sony 코드서명 인증서를 .cat 에서 추출
        $cat = Join-Path $stage "driver\srcameradriver.cat"
        if (Test-Path $cat) {
            $sig = Get-AuthenticodeSignature $cat
            if ($sig.SignerCertificate) {
                [IO.File]::WriteAllBytes((Join-Path $stage "driver\sony_codesign.cer"),
                                         $sig.SignerCertificate.Export("Cert"))
            }
        }
    } else {
        Write-Warning "winusb_driver\ 없음 — 드라이버는 패키지에서 제외됨(README 안내만 포함)"
    }
}

# 4) README.txt
if ($NoSony) {
    $readme = @"
TetherMoon (Windows x64) — portable

이 빌드에는 Sony Camera Remote SDK 파일이 포함돼 있지 않습니다(재배포 라이선스). 직접 배치하세요.

[1] Sony Camera Remote SDK(Windows)를 받아 RemoteCli.zip 을 풀고, external\crsdk 안의 다음을
    crsdk_server.exe 와 같은 폴더에 복사:
      - Cr_Core.dll, monitor_protocol.dll, monitor_protocol_pf.dll
      - CrAdapter\ 폴더 전체 (Cr_PTP_USB.dll, Cr_PTP_IP.dll, libusb-1.0.dll, libssh2.dll)

[2] libusbK 드라이버 설치: 카메라 USB 연결 + 카메라 메뉴에서 USB 원격(PC Remote) 켜기 →
    장치관리자 → ILCE-... 우클릭 → 드라이버 업데이트 → '직접 선택' → [디스크 있음] →
    SDK의 Driver.zip 안 srcameradriver.inf → '이 드라이버 설치'.

[3] 실행: crsdk_server.exe → 브라우저가 http://localhost:8080/web/index.html 로 열림.
    폰은 콘솔에 표시되는 http://<PC-IP>:8080/web/index.html (같은 Wi-Fi).

문제해결: 'no cameras detected' = SDK 미배치 / 드라이버 미설치 / 카메라 PC Remote 미설정.
종료는 웹 UI 종료 버튼(강제 종료 시 카메라 USB 재연결 필요).
"@
} else {
    $readme = @"
TetherMoon (Windows x64)
========================

[최초 1회: libusbK 드라이버 설치]
  1. 카메라를 USB로 연결하고, 카메라 메뉴에서 USB 원격(PC Remote)을 켠다.
  2. 장치관리자 → '휴대용 장치'/'카메라' 아래의 ILCE-... 우클릭 → 드라이버 업데이트
  3. '내 컴퓨터에서 찾아보기' → '직접 선택' → [디스크 있음] → driver\srcameradriver.inf
  4. 게시자 경고가 뜨면 '이 드라이버 설치'를 누른다.
  5. 장치가 'libusbK USB Devices / Sony Remote Control Camera' 로 바뀌면 성공.

[실행]
  crsdk_server.exe 더블클릭 → 기본 브라우저가 http://localhost:8080/web/index.html 로 열림.
  같은 Wi-Fi의 폰에서는 콘솔 창에 표시되는 http://<PC-IP>:8080/web/index.html 로 접속.

[종료]
  웹 UI의 종료 버튼, 또는 콘솔 창을 닫는다. (강제 종료 시 카메라를 USB 재연결해야 할 수 있음)

[문제 해결]
  - 'no cameras detected': 드라이버 미설치 또는 카메라 PC Remote 미설정.
  - 재연결이 ConnectTimeout: 카메라 USB를 뺐다 다시 연결.
"@
}
Set-Content -Encoding UTF8 -Path (Join-Path $stage "README.txt") -Value $readme

# 5) zip
$zip = Join-Path $root ("dist\" + $name + ".zip")
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path "$stage\*" -DestinationPath $zip
Write-Output ("packaged: " + $zip)
Write-Output ("staged  : " + $stage)
