# Screen Bright Controller

모니터별 또는 전체 화면을 어둡게 조절하는 Windows 트레이 유틸리티입니다. Rust와 Tauri 2로 만들었으며, 화면 오버레이나 DDC/CI 대신 GPU 감마를 조절합니다.

[English](README.md)

## 주요 기능

- 모니터별·연결 조절: **디밍 정도 0–90, 모든 정수**
- 슬라이더·키보드·휠·숫자 입력 즉시 적용
- 트레이 전용 조절 창, Display 1·Display 2 표시
- 종료 시 원래 감마 복원 및 별도 복원 감시 프로세스
- 헤더 ⚙ 설정의 **Start with Windows (Windows 시작 시 실행)**: 기본 꺼짐
- 일반 실행·로그인·중복 실행 모두 트레이에만 유지, 자동 창 열기·디밍·밝기 복원 없음

## 사용

[GitHub Releases](https://github.com/HanChangHun/ScreenBrightController/releases/latest)에서 최신 **`ScreenBrightController_<버전>_x64-setup.exe`**를 내려받아 설치합니다. 관리자 권한 없이 현재 사용자에게 설치되며 시작 메뉴·바탕 화면 바로가기와 Windows 설치된 앱 목록에 등록됩니다.

기본 설치 경로: `%LOCALAPPDATA%\Screen Bright Controller\ScreenBrightController.exe`. WebView2가 없을 때만 설치 프로그램이 내려받습니다. 코드 서명 인증서가 없는 배포판이므로 Windows에서 게시자/SmartScreen 경고가 나올 수 있습니다. 이 저장소의 배포 파일을 사용하고 필요하면 함께 제공되는 `SHA256SUMS.txt`와 비교하세요.

**설치만으로 자동실행이나 디밍이 켜지지 않습니다.** 시작 메뉴에서 실행한 뒤 **트레이 아이콘을 왼쪽 클릭**해 조절하세요. 별도 메인 창과 Open app 동작은 없습니다. 업데이트·제거 전에는 트레이의 **Restore and quit**으로 정상 종료해야 합니다. 설치 프로그램은 실행 중인 앱이나 복원 감시 프로세스를 강제 종료하지 않고 작업을 중단합니다. **자동 업데이트 기능은 없으며**, 새 버전 설치 파일을 직접 실행해 업데이트합니다. 개발 폴더 `dist`의 구형 포터블 실행 파일은 설치본에 필요하지 않으며 동시에 실행하면 안 됩니다.

변경은 즉시 적용되며 동의 체크박스·적용 버튼·확인 팝업이 없습니다. 드래그·숫자 입력은 1 단위, 슬라이더 방향키·휠은 ±5, Home/End는 0/90입니다. X·Escape·트레이 아이콘 재클릭으로 조절 창을 숨겨도 연속 디밍은 유지되며 **Restore original**로 복원합니다. 팝업은 포커스를 잃어도 유지됩니다. 트레이 아이콘 우클릭 메뉴에서 복원 또는 **Restore and quit**을 선택할 수 있습니다. 자동실행 설정은 팝업의 ⚙에 있습니다.

**0**은 원래 감마, **90**은 원래 감마 값의 10%를 요청합니다. 모니터 백라이트 밝기 10%를 뜻하지 않습니다. Windows x64와 Microsoft Edge WebView2 Runtime이 필요합니다.

Windows/드라이버가 요청을 거부하거나 읽기 검증에 실패하면 트레이 조절 창의 조작을 잠그고 대기 중인 요청을 버립니다. 이때 숫자는 **적용이 확인된 값이 아니라 요청한 값**입니다. 간단한 안내의 **Restore**로 복원한 뒤 더 낮은 값을 직접 시도하세요. 자동 재시도·임의 범위 제한·드라이버 제한 우회는 하지 않습니다. API 반환과 읽기 검증 결과는 **Details**에서 구분해서 확인할 수 있습니다. 복원 실패 시에는 조작 잠금과 복원 버튼을 유지하고, 상태 조회 실패 시에도 조회가 회복될 때까지 새 요청을 보내지 않습니다.

체크박스는 현재 사용자의 실제 Windows Run 등록을 읽습니다. 직접 변경할 때만 등록하고 결과를 다시 읽어 확인합니다. 실패하면 기존 등록 복구를 시도하고 실제 상태를 표시하며, 상태를 읽을 수 없으면 체크박스를 비활성화합니다. 관리자 권한은 필요 없습니다. 실행 파일은 고정 위치에 두세요. 이동·이름 변경 후 다시 켜면 경로가 갱신됩니다. Windows 시작 앱/작업 관리자 또는 조직 정책의 별도 차단은 이 설정으로 우회하지 않습니다.

## 빌드

```sh
npm --prefix app ci --ignore-scripts
npm --prefix app run build
cargo test --workspace --locked
node scripts/ui-smoke.cjs
node scripts/ui-settings.cjs
node scripts/ui-recovery.cjs
node scripts/ui-copy.cjs
python scripts/test_packaging.py
python scripts/test_tray_only.py
python scripts/test_branding.py
python scripts/verify-release.py
python scripts/verify-icon.py
python scripts/verify-continuous.py
```

결과: `target/release/ScreenBrightController.exe`와 `target/release/bundle/nsis/Screen Bright Controller_<버전>_x64-setup.exe`. Rust/MSVC, Tauri Windows 빌드 환경, Node/npm과 검증용 Python 3이 필요합니다.

소스 패키지는 `screen-bright-controller`(코어)와 `screen-bright-controller-tray`(Tauri 트레이)입니다. 별도 진단용 실행 타깃은 `screen-bright-controller-cli`이며 `cargo build --release --locked --bin screen-bright-controller-cli`로 빌드합니다. 아이콘 참조 파일의 접두사도 `screen-bright-controller`로 통일했습니다. 배포 실행 파일명·설치 식별자·제공된 ICO는 유지합니다. 감마 API 이름은 기술 용어이므로 제품명 변경 대상이 아닙니다.

검증 스크립트는 구형 `dist` 실행 파일을 다시 만들지 않습니다. `--exe "설치된 실행 파일 경로"`와 `--evidence-dir "검증 결과 폴더"`를 지정할 수 있습니다. 실제 감마 진단은 읽기 전용이며 watchdog 장애 검증은 메모리 백엔드만 사용합니다. 반복 가능한 배포 절차는 [docs/RELEASING.md](docs/RELEASING.md)에 있습니다.

검증 범위와 남은 수동 확인 사항은 [VALIDATION.md](VALIDATION.md)에 있습니다. 실행 파일과 로컬 진단 자료는 소스 저장소에 포함하지 않습니다.
