; scripts/installer.iss — TetherMoon Windows 인스톨러 (Inno Setup)
;
; 빌드 (repo 루트에서, package-win.ps1 로 dist\TetherMoon-win-x64\ 먼저 생성):
;   iscc scripts\installer.iss
; 산출물: dist\TetherMoon-setup.exe
;
; 설치 시: 앱 파일 → Program Files, libusbK 드라이버 자동 설치(관리자), 바로가기/제거 등록.

#define AppName "TetherMoon"
#define AppVersion "0.2.5"
#define AppExe "crsdk_server.exe"
#define StageDir "..\dist\TetherMoon-win-x64"

[Setup]
AppId={{B7F3A1C2-9D45-4E68-A0F1-2C3D4E5F6A7B}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=SpaceDLFactory
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
OutputDir=..\dist
OutputBaseFilename=TetherMoon-setup
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
UninstallDisplayIcon={app}\{#AppExe}

[Languages]
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; Flags: unchecked

[Files]
Source: "{#StageDir}\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{group}\{cm:UninstallProgram,{#AppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
; 1) Sony 코드서명 인증서를 TrustedPublisher 에 등록(드라이버 무인 설치 위해)
Filename: "certutil.exe"; \
  Parameters: "-f -addstore TrustedPublisher ""{app}\driver\sony_codesign.cer"""; \
  StatusMsg: "{cm:DrvCert}"; Flags: runhidden waituntilterminated; \
  Check: FileExists(ExpandConstant('{app}\driver\sony_codesign.cer'))
; 2) libusbK 드라이버 설치 (카메라를 PC Remote 모드로 연결한 뒤 재연결 시 바인딩)
Filename: "pnputil.exe"; \
  Parameters: "/add-driver ""{app}\driver\srcameradriver.inf"" /install"; \
  StatusMsg: "{cm:DrvInstall}"; Flags: runhidden waituntilterminated; \
  Check: FileExists(ExpandConstant('{app}\driver\srcameradriver.inf'))
; 3) 설치 후 실행 옵션
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; \
  Flags: nowait postinstall skipifsilent

[CustomMessages]
korean.DrvCert=드라이버 인증서 등록 중...
korean.DrvInstall=카메라 드라이버(libusbK) 설치 중...
english.DrvCert=Registering driver certificate...
english.DrvInstall=Installing camera driver (libusbK)...
