# Screen Bright Controller v0.3

Windows x64 gamma dimming utility built with Rust / Tauri 2. Product name is **Screen Bright Controller** (not “Brightness”). Internal crate names remain `gamma-dimmer`.

## 새 디자인 / 조작

탭이나 수직 M/1/2 패널 없이 charcoal / restrained teal의 **Operate** 화면을 사용합니다. 위의 **Link displays**로 포함된 디스플레이를 연결하고, 아래의 수평 모니터 행에서 각각 조작합니다. 실제 모드에서는 장치 경로를 표시하여 같은 어댑터의 디스플레이도 구분합니다. main과 tray popup은 Rust backend의 값, 포함 여부, 동의, 결과를 공유합니다.

1. HDR을 직접 끄고 Night Light / ICC / 다른 dimmer 등 색 보정 도구를 비활성화합니다. HDR 자동 감지는 없습니다.
2. 연결 또는 개별 모드에서 **0–90, 정확히 5 단위**로 값을 준비합니다. 0은 시작 시 저장된 원래 감마, 90은 그 감마의 10%입니다. 백라이트 밝기가 아닙니다.
3. 동의 체크 → **Apply continuous** → 확인 대화상자를 거쳐야 지속 적용됩니다. 적용할 때마다 동의를 다시 받아야 합니다. 슬라이더의 비영점 변경은 값만 준비하고 감마를 SET하지 않습니다.
4. **Preview 15s**는 별도의 일회성 미리보기입니다. 15초 후 복원되며 heartbeat로 연장하지 않습니다. 미리보기를 먼저 복원해야 지속 모드로 전환할 수 있습니다.
5. 지속 모드의 이미 적용된 디스플레이는 다시 Apply하여 안전하게 값을 바꿀 수 있습니다. 원본 스냅샷은 새로 캡처하지 않습니다.
6. 활성 대상의 값 0 또는 제외 체크는 해당 대상을 즉시 원본으로 복원합니다. 연결된 값 0은 활성 대상을 복원합니다. 처음부터 0인 미사용 대상은 arm / SET / restore를 하지 않습니다.
7. **Restore original**은 시도한 대상만 복원하고, 성공 시 main/popup의 모든 dim 값을 0으로 초기화합니다. 복원 실패 시 상태와 원본을 보존하고 오류를 표시합니다.

**지속 모드에서 main 닫기 = 트레이로 숨기기, 계속 유지.** 미리보기의 main 닫기는 복원 후 숨깁니다. 트레이 왼쪽 클릭은 430×540 logical px의 빠른 조작 팝업, 오른쪽 클릭은 Open / Restore / Restore and quit 메뉴입니다. popup의 Escape / × / 포커스 손실은 숨기기만 합니다. **Quit은 복원 후 종료하며 복원 실패 시 종료를 취소합니다.** Tab / 방향키 / Home / End / 숫자 입력 및 명확한 focus outline을 제공합니다.

실행 시 자동 적용, 다음 실행 값 저장, autostart 등록, 레지스트리 변경, 서비스 설치는 **하지 않습니다**. 기본값 0 / 동의 해제로 읽기 전용 시작합니다. 상태 조회, 트레이 열기, 두 번째 실행은 적용하지 않습니다. 동일 identifier의 새 앱은 single-instance이며 구버전 dimmer는 사용자가 먼저 안전하게 복원 / 종료해야 합니다.

## 지속 모드 보호

- 독립 watchdog이 **자신의 Win32 읽기**로 원래 RGB 감마를 캡처하여 메모리에 소유합니다. 외부 감마 배열 / 임의 스냅샷 파일은 받지 않습니다.
- 앱의 native worker가 webview와 독립적으로 **2초마다** health/readback 확인 및 **10초 lease** 갱신을 수행합니다. 숨긴 창에서도 유지합니다. heartbeat는 gamma SET을 반복하지 않습니다.
- watchdog은 약 100ms마다 deadline, 부모 Windows process handle, stdin EOF를 확인합니다. lease 만료, 부모 사망, pipe EOF는 원본 복원을 실행합니다. 만료한 lease나 미리보기는 갱신할 수 없으며 실패한 복원의 재시도도 heartbeat로 취소할 수 없습니다.
- OS 난수 인증 Hello 및 전용 파이프를 사용합니다. `Arm(id, seconds)`는 1–30초, UI 미리보기는 15초 고정. `Continuous(id, lease)`와 `Renew(id, lease)`는 lease 10만 허용합니다. 알 수 없는 필드 / ID / 중복 arm을 거부합니다. `Restore`, `Status`, `Quit` 외의 임의 snapshot / ramp 명령은 없습니다. 같은 사용자 권한의 악의적 프로그램에 대한 보안 경계로 주장하지 않습니다.
- 지속 모드에서 외부 색 변화, readback mismatch/error가 발견되면 해당 대상은 저장된 원본으로 복원 / disarm하고 경고합니다. 다른 프로그램과 경쟁하여 감마를 다시 강제 적용하지 않습니다. 새 Apply는 다시 동의해야 합니다. API/readback 결과는 상세 영역에서 분리 표시하며 화면의 시각적 효과를 보증하지 않습니다.
- 복원 실패는 스냅샷을 유지하고 재시도합니다. 부모 생존 중 계속 재시도, 부모 사망 / EOF 후 최대 60초 재시도. 부모가 watchdog을 kill하지 않습니다.

## 실행 / 빌드

로컬 빌드 산출물은 `dist/ScreenBrightController-v0.3.exe`입니다. 구버전 실행 파일을 덮거나 실행 중 프로세스를 종료하지 않습니다. Microsoft Edge WebView2 Runtime이 필요하며 .NET / Python / Node는 실행에 필요하지 않습니다. watchdog은 `current_exe()`로 같은 실행 파일을 사용합니다.

```bash
cargo build --release --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
node --check app/ui/app.js
node --check app/ui/demo.js
node scripts/ui-smoke.cjs
# 아래 검증은 native 읽기 / memory mock만 수행합니다.
python scripts/verify-release.py
python scripts/verify-icon.py
python scripts/verify-continuous.py
```

```bash
./dist/ScreenBrightController-v0.3.exe             # 읽기 전용 시작, 실제 적용은 사용자 UI 조작만
./dist/ScreenBrightController-v0.3.exe --demo      # memory-only UI
./dist/ScreenBrightController-v0.3.exe --diagnose > diagnose.json
./dist/ScreenBrightController-v0.3.exe --startup-check > startup.json
./dist/ScreenBrightController-v0.3.exe --self-test > mock.json
```

`--mock` / `--self-test`는 CLI memory test, `--demo`는 UI memory mode입니다. 실제 감마를 적용하는 공개 CLI 명령은 없습니다. watchdog 내부 모드는 부모 환경과 인증 없이 수동 실행할 수 없습니다.

## Browser-only 디자인 검증

```bash
python -m http.server 8766 --bind 127.0.0.1 --directory app/ui
# http://127.0.0.1:8766/harness.html
```

실제 main/popup HTML/CSS/JS를 browser-only memory bridge로 렌더링합니다. `demo.js`는 기존 Tauri bridge를 덮지 않으며 Win32 / Rust에 접근하지 않습니다. browser demo만 localStorage를 공유하고 native 값은 디스크에 저장하지 않습니다. browser 지속 모드는 native lease를 구현한 것처럼 주장하지 않으며 독립 watchdog은 별도로 실행 검증합니다. 캡처는 UI 디자인 확인용이며 gamma 효과 증거가 아닙니다.

## 한계 / 안전

HDR / Advanced Color에서 legacy gamma는 정의되지 않거나 무시될 수 있습니다. **HDR에서 적용하지 마세요.** Night Light / ICC / 게임 / GPU 서비스가 감마를 바꿀 수 있고 저장된 시작 원본 복원은 이후 외부 보정을 덮을 수 있습니다. 강한 dimming은 거절되거나 조용히 무시될 수 있습니다. 하드웨어 커서도 GPU 경로에 따라 다르며 screenshot은 감마 효과를 증명하지 않습니다.

단위는 Windows 활성 GDI display 경로입니다. 복제 모드에서 물리 화면별 독립 제어가 불가능할 수 있습니다. **활성 중 hotplug / monitor 교체 / topology 변경은 지원하지 않습니다.** 먼저 Restore / Quit 후 구성을 바꾸세요. Windows display-ID 재사용, 늦거나 막힌 driver API, health 확인과 write 사이의 race는 완전히 제거할 수 없습니다. OS / GPU reset / 전원 손실 / 두 프로세스 모두 종료 시 복원은 best effort입니다.

## Source-only 공개 저장소

`Cargo.lock`, 소스, 테스트, 문서, 원본 아이콘을 포함합니다. `target`, `dist`, `evidence`, generated schema, node_modules, 환경 파일과 로그는 `.gitignore`로 제외합니다. 로컬 캡처 / 읽기 snapshot / 바이너리 / 개인 상태는 공개 소스로 포함하지 않습니다. 설치 프로그램, 서명, release 업로드는 수행하지 않습니다.

사용자가 제공한 `app/src-tauri/icons/icon.ico`는 byte-for-byte 보존합니다. SHA-256: `1cbe4d747d8ef3e26840a6b700b48e83a3440fa333fbf6c994114acea72f6398`. 실행 파일의 7개 icon payload와 window PNG / tray RGBA가 이 원본의 entry와 일치합니다. 오래된 아이콘 생성 스크립트로 덮지 마세요. Tauri custom-protocol 기본 feature로 정적 UI를 EXE에 내장합니다.

구현은 독립 작성이며 공개 API 참고: https://github.com/SalvatoreSorvillo/VistaCare (MIT). 검증 범위와 남은 수동 acceptance는 `VALIDATION.md`를 확인하세요.
