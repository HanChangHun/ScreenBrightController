# Screen Bright Controller

모니터별 또는 전체 화면을 어둡게 조절하는 Windows 트레이 유틸리티입니다. Rust와 Tauri 2로 만들었으며, 화면 오버레이나 DDC/CI 대신 GPU 감마를 조절합니다.

[English](README.md)

## 주요 기능

- 모니터별·연결 조절: **디밍 정도 0–90, 모든 정수**
- 슬라이더·키보드·휠·숫자 입력 즉시 적용
- 메인·트레이 값 공유, Display 1·Display 2 표시
- 종료 시 원래 감마 복원 및 별도 복원 감시 프로세스
- 헤더 ⚙ 설정의 **Start with Windows (Windows 시작 시 실행)**: 기본 꺼짐
- 로그인 시 트레이로만 실행, 자동 디밍·밝기 복원 없음

## 사용

[GitHub Releases](https://github.com/HanChangHun/ScreenBrightController/releases/latest)에서 최신 **`ScreenBrightController_<버전>_x64-setup.exe`**를 내려받아 설치합니다. 관리자 권한 없이 현재 사용자에게 설치되며 시작 메뉴·바탕 화면 바로가기와 Windows 설치된 앱 목록에 등록됩니다.

기본 설치 경로: `%LOCALAPPDATA%\Screen Bright Controller\ScreenBrightController.exe`. WebView2가 없을 때만 설치 프로그램이 내려받습니다. 코드 서명 인증서가 없는 배포판이므로 Windows에서 게시자/SmartScreen 경고가 나올 수 있습니다. 이 저장소의 배포 파일을 사용하고 필요하면 함께 제공되는 `SHA256SUMS.txt`와 비교하세요.

**설치만으로 자동실행이나 디밍이 켜지지 않습니다.** 시작 메뉴에서 실행한 뒤 직접 조절하세요. 업데이트·제거 전에는 트레이의 **Restore and quit**으로 정상 종료해야 합니다. 설치 프로그램은 실행 중인 앱이나 복원 감시 프로세스를 강제 종료하지 않고 작업을 중단합니다. **자동 업데이트 기능은 없으며**, 새 버전 설치 파일을 직접 실행해 업데이트합니다. 개발 폴더 `dist`의 구형 포터블 실행 파일은 설치본에 필요하지 않으며 동시에 실행하면 안 됩니다.

변경은 즉시 적용되며 동의 체크박스·적용 버튼·확인 팝업이 없습니다. 드래그·숫자 입력은 1 단위, 슬라이더 방향키·휠은 ±5, Home/End는 0/90입니다. 메인 창을 닫아도 트레이에서 유지되며 **Restore original**로 복원합니다. 트레이 팝업은 포커스를 잃어도 유지되고 X·Escape·트레이 토글·Open app으로만 숨깁니다.

**0**은 원래 감마, **90**은 원래 감마 값의 10%를 요청합니다. 모니터 백라이트 밝기 10%를 뜻하지 않습니다. Windows x64와 Microsoft Edge WebView2 Runtime이 필요합니다.

체크박스는 현재 사용자의 실제 Windows Run 등록을 읽습니다. 직접 변경할 때만 등록하고 결과를 다시 읽어 확인합니다. 실패하면 기존 등록 복구를 시도하고 실제 상태를 표시하며, 상태를 읽을 수 없으면 체크박스를 비활성화합니다. 관리자 권한은 필요 없습니다. 실행 파일은 고정 위치에 두세요. 이동·이름 변경 후 다시 켜면 경로가 갱신됩니다. Windows 시작 앱/작업 관리자 또는 조직 정책의 별도 차단은 이 설정으로 우회하지 않습니다.

## 빌드

```sh
npm --prefix app ci --ignore-scripts
npm --prefix app run build
cargo test --workspace --locked
node scripts/ui-smoke.cjs
node scripts/ui-settings.cjs
python scripts/test_packaging.py
python scripts/verify-release.py
python scripts/verify-icon.py
python scripts/verify-continuous.py
```

결과: `target/release/ScreenBrightController.exe`와 `target/release/bundle/nsis/Screen Bright Controller_<버전>_x64-setup.exe`. Rust/MSVC, Tauri Windows 빌드 환경, Node/npm과 검증용 Python 3이 필요합니다.

검증 스크립트는 구형 `dist` 실행 파일을 다시 만들지 않습니다. `--exe "설치된 실행 파일 경로"`와 `--evidence-dir "검증 결과 폴더"`를 지정할 수 있습니다. 실제 감마 진단은 읽기 전용이며 watchdog 장애 검증은 메모리 백엔드만 사용합니다. 반복 가능한 배포 절차는 [docs/RELEASING.md](docs/RELEASING.md)에 있습니다.

검증 범위와 남은 수동 확인 사항은 [VALIDATION.md](VALIDATION.md)에 있습니다. 실행 파일과 로컬 진단 자료는 소스 저장소에 포함하지 않습니다.
