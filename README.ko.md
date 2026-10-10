# Screen Bright Controller

Windows 트레이에서 모니터별 또는 전체 화면을 어둡게 조절하는 유틸리티입니다.

[다운로드](https://github.com/HanChangHun/ScreenBrightController/releases/latest) · [English](README.md)

## 주요 기능

- 모니터별·전체 디밍 조절, 0–90의 모든 정수 지원
- 슬라이더·키보드·휠·숫자 입력 즉시 적용
- 이동·크기 조절이 가능한 작은 다크 테마 트레이 창
- 종료 시 원래 감마 복원 및 별도 복원 감시 프로세스
- 선택 가능한 Windows 시작 시 실행, 기본 꺼짐

## 시작하기

1. [최신 릴리스](https://github.com/HanChangHun/ScreenBrightController/releases/latest)에서 `ScreenBrightController_<버전>_x64-setup.exe`를 내려받아 설치합니다.
2. 시작 메뉴에서 실행한 뒤 트레이 아이콘을 왼쪽 클릭합니다.
3. 슬라이더로 조절합니다. **Restore original**로 복원하고, 종료할 때는 트레이 우클릭 메뉴의 **Restore and quit**을 선택합니다.

Windows x64가 필요합니다. 관리자 권한 없이 설치하며, WebView2가 없으면 설치 파일에 포함된 Microsoft 설치 도구로 함께 설치합니다(인터넷 연결 필요). 실행만으로 디밍이 적용되지 않으며, 조절 창을 숨겨도 적용 중인 디밍은 유지됩니다.

## 알아둘 점

- 모니터 백라이트 밝기가 아닌 GPU 감마를 조절합니다.
- 드라이버 제한과 HDR/ICC/Night Light의 영향을 받으며, 모든 환경에서 0–90 전체 범위의 효과를 보장하지 않습니다.
- 알려진 한계: 하드웨어 커서는 밝게 남을 수 있고, 드라이버가 변경을 늦게 반영할 수 있으며, 디밍 중 모니터 연결 변경·디스플레이 ID 재사용·GPU/OS 재설정은 지원하지 않습니다. API 성공이나 읽기 확인 일치가 실제 화면 효과를 보장하지 않으며, 네이티브 호출이 멈추거나 앱과 감시 프로세스가 모두 종료되면 복원은 최선의 시도에 그칩니다.
- 오류로 조작이 잠기면 **Restore**로 복원한 뒤 더 낮은 값을 시도하세요.
- 실행 시 저장된 감마가 이미 정상보다 크게 어두우면(예: 앱과 감시 프로세스가 디밍 중 함께 종료된 경우) 조절 창에 경고와 **Reset to default ramp** 버튼이 나타납니다. 보정(ICC) 감마는 원래 선형이 아닐 수 있으므로 자동으로 초기화하지 않습니다.
- 업데이트는 수동입니다. 업데이트·제거 전에는 **Restore and quit**으로 종료하세요.
- 업데이트해도 **Start with Windows** 설정은 유지됩니다. 제거할 때도 지워지지 않으니, 제거 전에 조절 창 설정에서 꺼 주세요.
- 서명되지 않은 설치 파일이라 SmartScreen 경고가 나올 수 있습니다. [설치 안내](docs/USAGE.ko.md#설치와-업데이트)를 참고하세요.

## 빌드

Rust와 Tauri 2로 만들었습니다. Windows에 Rust/MSVC, Node/npm과 Tauri Windows 빌드 환경을 갖춘 뒤 실행합니다.

```sh
npm --prefix app ci --ignore-scripts
npm --prefix app run build
```

테스트·결과 파일·패키징 절차는 [배포 가이드](docs/RELEASING.md)에 있습니다.

## 문서

[사용 가이드](docs/USAGE.ko.md) · [수동 확인](MANUAL_TESTS.md) · [문제 제보](https://github.com/HanChangHun/ScreenBrightController/issues)
