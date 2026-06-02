# STATUS — CrSDK 테더링 서버 기능 현황

최종 갱신: 2026-05-31. 구조 상세는 `ARCHITECTURE.md`, 로드맵은 `PLAN.md` 참조.

## 1. 아키텍처 한눈에

```
[Sony A7C] ──USB(PTP, Remote 모드)── [Mac: crsdk_server (axum)] ──HTTP/SSE/MJPEG── [브라우저/폰 UI]
```

- **lib `crsdk`** (동기 FFI 래퍼): session·enumerate·connection·callback·liveview·shutter·properties·control·error·ffi
- **`crsdk_server`** (axum, tokio): 모든 SDK 호출은 `spawn_blocking`으로 격리, 카메라는 `Arc<Mutex<Option<CameraCell>>>`('static newtype + unsafe Send)
- **`web/index.html`**: 단일 페이지 (B·Tether Console), 2초 폴링 + SSE

## 2. HTTP/SSE 엔드포인트 (구현됨)

| 경로 | 역할 |
|------|------|
| `GET /api/status` | 연결상태·model·handle·save_path·lens_model |
| `POST /api/connect` `/disconnect` | 연결 (PriorityKey=PCRemote + set_save_info 자동) |
| `GET /api/properties` | 노출·포커스·드라이브·측광·파일포맷·배터리·남은컷 등 |
| `POST /api/property {code,value}` | 속성 쓰기 (Fetch-Modify-Set) |
| `POST /api/shutter` | 단발 (focus mode 감지: MF→capture / AF→capture_af) |
| `POST /api/shutter/down` `/up` | press-hold 연사 |
| `POST /api/movie/start` `/stop` | 동영상 녹화 토글 |
| `POST /api/cancel` | 촬영 취소 |
| `POST /api/focus_nearfar {step}` + `GET /info` | MF 초점 이동(NearFar), 범위는 카메라가 제공 |
| `POST /api/af_point {x,y}` | AF 영역 좌표 (정규화→패킹, 기본 중앙) |
| `POST /api/savepath {path,prefix}` | PC 저장 폴더 + 파일명 접두사 |
| `GET /api/last_image` | 마지막 PC 저장 이미지 (미리보기) |
| `GET /events` (SSE) | DownloadComplete·PropertyChanged·Disconnected·Error |
| `GET /lv` (MJPEG) | LiveView 영상 |
| `GET /api/capabilities` | 연결 바디의 model + 노출 property code 집합 (프론트 UI 큐레이션) |
| `GET /api/_debug/codes` | 카메라가 보고하는 전체 property code 덤프 (진단용) |

## 3. 구현 완료 기능

### 연결·세션
- ✅ 자동 연결 + USB 간섭 억제(ptpcamerad) + PriorityKey=PCRemote
- ✅ **자동 재연결** — 미연결 시 3초 폴링 + Disconnected 이벤트 시 상태 비움
- ✅ 모델명·핸들·렌즈모델명(지원 바디 한정) 표시

### 라이브뷰
- ✅ MJPEG 실시간 영상
- ✅ **포커스 피킹** (클라이언트 canvas 엣지검출)
- ✅ 3분할 그리드 오버레이(상시)

### 노출·색 (전부 카메라 allowed 기반 dropdown)
- ✅ ISO / 셔터 / 조리개 / EV / WB / 측광 / 드라이브 / 파일포맷(RAW/JPEG/HEIF)
- ✅ 셔터타입·사일런트 (A7C 미지원 시 자동 비활성)
- ✅ **부정확/쓰레기 값 필터** — 디코더가 매핑 실패한 SDK 패딩·쓰레기값을 dropdown에서 제외 (drive/iso/조리개/셔터 실데이터 검증 완료)

### 포커스
- ✅ MF/AF 모드 + **NearFar 슬라이더**(카메라가 min/max/step 제공, ±7)
- ✅ AF 셔터(S1 시퀀스)
- ✅ **AF 포인트 지정** (LiveView 클릭, 기본 중앙) — 좌표 인코딩은 추정값, 하드웨어 미검증

### 촬영
- ✅ 단발 / **연사**(press-hold) / 동영상 녹화 토글 / 촬영 취소
- ✅ 셀프타이머 (DriveMode dropdown 재사용)
- ✅ **장노출** 하드웨어 검증: 고정 1"~30"(셔터 dropdown, capture로 동작 — RAW 다운로드 ~8s 지연 있음) + **BULB**(셔터값 0). 벌브는 종전 0-필터로 dropdown에서 막혀 있던 걸 `dec.shutter(0)='BULB'` + `allowZero`로 해제, CAPTURE 누르고 유지=노출시간으로 홀드 촬영 확인
- ✅ **벌브 타이머**(소프트웨어) `/api/bulb {seconds}` — A7C는 네이티브 벌브타이머(0x0209) 미노출이라, 서버가 셔터 BULB로 두고 down→sleep(N)→up 타이밍을 직접 잼(1~900s). 백그라운드 tokio 태스크 + `bulb_active` AtomicBool로 중복 트리거 409 거부. JS는 카운트다운 표시. 30초 초과 장노출용. 하드웨어 검증 완료(3s/2s 왕복, 중복거부)

### 저장·상태
- ✅ RAW/JPEG/HEIF, 저장대상(SD/PC/PC+SD), PC 폴더 변경, **파일명 접두사**
- ✅ 저장 완료 알림(SSE 토스트) + **촬영 미리보기**(PC 저장 JPEG)
- ✅ **배터리 잔량 %** + 남은 컷(MediaSLOT1) 표시

## 4. 알려진 이슈 / 미검증

- ✅ ~~AF 포인트 좌표 인코딩 추정~~ → 해결: device property 0x0121, x:0~639/y:0~479 (5절 참조)
- ⚠️ **하드웨어 미검증 누적**: 연사·동영상·촬영취소·셔터타입·사일런트·미리보기·피킹·AF포인트·배터리표시 (`SHUTTER_TEST.md` 체크리스트)
- ℹ️ 남은 컷 = 0 은 PC 저장 모드(SD 미사용) 시 정상
- ℹ️ shutter_type/silent_mode = A7C 미노출(null) → dropdown 비활성

## 5. 구현 예정 (우선순위)

### Tier 2 — 촬영/포커스 확장 ★ 현재 작업 대상
- ✅ **JPEG 품질**(StillImageQuality 0x0107) dropdown — 하드웨어 검증 완료(읽기+쓰기 왕복). allowed=[X.Fine/Fine/표준]
- ✅ **Picture Profile**(0x01AA) dropdown — 하드웨어 검증 완료(0→3→0 왕복). Off+PP1~PP10. Off(0)이 실제 값이라 `fillSelect(...,allowZero=true)`로 0-필터 우회
- ✅ **WB 켈빈 슬라이더** (Colortemp 0x0115) — 하드웨어+브라우저 검증 완료. value=생 켈빈(5500), WB=색온도(256)일 때만 editable→슬라이더 활성. 범위 **하드코딩 2500~9900/100K**(옵션 A), 실기에서 경계·중간값 쓰기 수용 + UI 활성화 확인
- ✅ **AF 영역 좌표 보정** — 실측+공식샘플로 확정. device property `AF_Area_Position`(0x0121, UInt32 `(x<<16)|y`, **x:0~639 y:0~479**) 사용. 종전 control code(0xD2DC)+0~10000은 오류였음. 좌표 지정 전 FocusArea=Flexible_Spot_S 자동 전환. 박스 위치 시각 확인 완료
- ✅ **반셔터(S1) 버튼** — 누르고 유지=`S1 LOCKED`(AF 합초·고정), 떼면 UNLOCKED. 연사와 동일한 pointer press-hold. CAPTURE는 자체 AF 시퀀스 유지(반셔터는 사전 합초·확인용)
- ✅ **AF 박스 크기 S/M/L** — af_point에 area 파라미터(FocusArea 0x04/05/06). 실기 3종 수용 확인(200), 잘못된 값은 S 폴백
- ✅ **라이브뷰 회전 토글**(수동, ↻ 90°씩, lv-img CSS rotate+scale fit) — 세로 촬영 시 정위치. ⚠️ **자동회전(자이로) 불가**: A7C는 `CrLiveViewProperty_Level` 미노출(라이브뷰 켠 상태에서도 InvalidCalled 0x8402 확인). ✅ 그리드도 라이브뷰와 동반 회전. ✅ AF 클릭 좌표 회전 리맵(`unrotateClick`): 화면 클릭에 `rotate(lvRot)·scale(s)` 역변환 적용→미회전 센서 좌표로 전송, 마커는 클릭한 화면 위치 표시(0°/90°/180°/270° 수치 검증). 미검증: 하드웨어
- ✅ **Graceful shutdown** — SIGTERM/SIGINT 잡아 종료 전 카메라 disconnect. 종전엔 pkill(SIGTERM 미처리)로 세션이 남아 재연결 시 FailBusy(0x820B)→power cycle 필요했음. 이제 재시작 즉시 연결 검증됨
- ✅ **AF 박스 클릭 정밀도** — `liveview_get_af_frame` FFI(0x0121 LV property)로 실위치 readback. 측정: X 선형(중앙 75%), Y는 S커브(중앙 압축, 0.25→0.14). `calib_y` 역보정(5점 실측표 역보간)으로 Y 선형화 → 하드웨어+시각 검증 완료(등간격 ~67, "괜찮아보임"). ⚠️ 보정표는 세션 간 ~4.5% offset 변동 가능(하드코딩 한계, 허용 수준)
- ✅ **반셔터(S1)** — 정상 동작 확인: half/down→FocusIndication 1(Unlocked)→258(Focused-AFS), half/up 후 합초 유지. 이전 "불완전"은 렌즈 MF 스위치/대상 상태 문제였음(코드 정상). FocusIndication(0x707) 계기판으로 검증
- ℹ️ 진단 엔드포인트: `/api/_debug/level`(자이로, A7C 미지원이나 **타 바디용으로 유지** — 프로젝트가 멀티바디 지향), `/api/_debug/afframe`(AF 실위치, 동작)
- 📌 방향: **A7C 전용 아님 → 차후 다른 카메라도 연결 예정.** A7C 미지원 기능도 코드는 유지(제거 금지), 런타임에 노출 여부로 분기
- ⚠️ 운영 교훈: 서버 재시작 시 **중복 프로세스** 주의(둘이 카메라 물면 ConnectTimeout 0x8208). graceful shutdown으로 단일 종료는 해결됨
- ✅ **인터벌/타임랩스** — A7C는 내장 인터벌 설정 미노출 → **소프트웨어 인터벌**(`/api/interval {interval_sec,count}` + `/stop`, 백그라운드 루프, 1초 단위 취소). 하드웨어 검증(2장 촬영, 409 중복거부, 정지/재시작). MF 권장, RAW는 간격 ≥10초
- 🔧 **dropdown 라벨 중복 제거** — fillSelect가 같은 라벨(연속브라켓/싱글브라켓/연속타이머 등 변종)을 1개로 접음. 카메라가 보고하는 변종 도배 해소
- 🚫 A7C 미지원 확인됨(덤프 대조): RAW압축(0x0131), Creative Look(0x01C5) — 둘 다 카메라가 property 자체를 노출 안 함. SDK에 Creative Style 대체 property 없음. ✅ WB AWB(0) 0-필터 버그 수정: `fillSelect(selWb,...,allowZero=true)`로 현재값이 AWB 아니어도 드롭다운에 노출 (벌브·PP Off와 동일 처리)

### Tier 3 — 뷰·편의
- ✅ **그리드 토글** — 라이브뷰 우상단 ▦ 버튼, 3분할 그리드 on/off(기본 ON, `.lv-thirds` display)
- ✅ **히스토그램** — 우측 패널 카드(라이브뷰 위 오버레이 X, 어두운 장면 시인성). 라이브뷰 프레임 240×160 다운샘플 → RGB 256-bin, 가산합성(lighter) 렌더, setInterval ~8fps, 피킹과 별도 캔버스. 미검증: 하드웨어(라이브뷰 피드 필요)
- ❌ 100% 확대 초점확인, 촬영 히스토리/필름스트립, LiveView 다중 클라이언트

### 마지막 순위 — WiFi/SSH 연결
- ❌ **WiFi/SSH 연결** — lib에 연결 인증 경로(`ConnectMode::Wifi`+`get_fingerprint`)는 있으나, ① 카메라 발견이 `EnumCameraObjects` 자동탐색뿐(IP 등록 `CreateCameraObjectInfoEthernetConnection` FFI 미구현) ② A7C의 SDK WiFi 테더링 지원 여부 미검증. 착수 전 자동발견 실측 진단(경로 A/B 분기) 필요

### 별도 트랙 — 하드웨어 검증 (코딩 아님)
- 미검증 누적분 실측 필요(`SHUTTER_TEST.md`): 연사·동영상녹화·촬영취소·셔터타입·사일런트·미리보기·피킹·AF포인트·배터리표시
- 우선순위 의견: 추정 기반 기능(특히 AF 좌표)을 실기로 확인하는 게 신규 기능 추가보다 우선

### 제외 확정
- **동영상 PC 로컬 저장** — 동영상은 PC 직접 저장 프로퍼티가 SDK에 없음(`CrRecordingMediaMovie`=Slot만, `StillImageStoreDestination`은 정지영상 전용). PC 저장하려면 `CrSdkControlMode_ContentsTransfer`/`RemoteTransfer`로 모드 전환(Remote 연결 해제→재연결) 후 전송 API 필요. 라이브뷰·세션 단절 비용 대비 가치 낮아 보류
- 줌 조작(렌즈 전동줌 아님), 포커스 절대위치/거리/렌즈명(A7C 미지원 — `/api/_debug/codes` 덤프로 확인)

## 6. 빌드/실행

```bash
cd crsdk_rust_wrapper
pkill -KILL ptpcamerad        # 서버 내장 억제기 있으나 부팅 전 1회 권장
cargo run -p crsdk_server     # http://localhost:8080/web/index.html
```
(clang 21 시스템 Xcode 사용 시 SDKROOT 불필요)

## 7. 바디 추상화 설계 (다음 작업 — capability 레이어)

**동기**: A7C 미지원 분기가 산발(null 체크), AF 보정표·좌표범위가 A7C 하드코딩, 프론트 드롭다운 큐레이션 부재. 멀티바디·오픈소스 위해 정리.

**현황**: 이미 `find(code)`→Option/None = 런타임 capability(바디가 property 노출하나). 이걸 명시화 + 모델별 보정 + 프론트 큐레이션으로 끌어올림.

**계획(증분)**:
1. ✅ lib `src/capability.rs`: `Capabilities { model, supported: BTreeSet<u32> }` + `probe(handle, model)`(get_all로 코드 수집) + `has(code)`. 토대.
2. ✅ 서버 `/api/capabilities` 엔드포인트 (model + supported codes hex) → 프론트가 UI 큐레이션.
3. ✅ **AF 보정 모델별 키화**: `AfCalib{x_max,y_max,y_cal}` + `af_calib(model)`. A7C(`ILCE-7C`)=실측표(`A7C_Y_CAL`), 미측정 바디=선형 폴백. `af_point`가 연결 모델로 보정.
4. ✅ 프론트: 연결당 1회 `/api/capabilities` → `data-code` 보유 컨트롤의 미지원 행 숨김 + property-row만의(버튼 없는) 카드 자동 숨김. 재연결 시 재큐레이션(`capsApplied`). A7C에선 shutter_type/silent 등 미노출 행이 비활성 '—' 대신 사라짐. (주: label-dedup은 한 property의 allowed 변종 도배 해소용이라 code-level capability와 별개 — 유지)
5. ⏸ 보류: 소프트웨어 폴백(벌브타이머·인터벌)을 "네이티브 미지원" 분기로. 네이티브 제어 UI가 없는 현재 단계에서 분기는 대체 없는 죽은 코드 → 네이티브 지원 바디 + 네이티브 UI 구현 시 함께.

**하드코딩 지점**(리팩터 대상): `crsdk_server/src/main.rs` AF_Y_CAL/calib_y/639/479/FLEXIBLE_SPOT, `src/properties.rs` 코드 주석(A7C 노출/미노출), PropertiesDto 21× find(), web/index.html 29× capability 분기.
