# Screen Bright Controller

모니터별 또는 전체 화면을 어둡게 조절하는 Windows 트레이 유틸리티입니다. Rust와 Tauri 2로 만들었으며, 화면 오버레이나 DDC/CI 대신 GPU 감마를 조절합니다.

[English](README.md)

## 주요 기능

- 모니터별·연결 조절: **디밍 정도 0–90, 5 단위**
- 지속 적용 및 15초 미리보기
- 메인 창과 트레이 팝업의 조절값 공유
- 종료 시 원래 감마 복원 및 별도 복원 감시 프로세스
- 시작 시 자동 디밍 없음

## 사용

로컬 빌드의 `dist/ScreenBrightController-v0.3.exe`를 실행합니다. 값을 선택하고 동의한 뒤 **Apply continuous** 또는 **Preview 15s**를 누르세요. 지속 적용 중 메인 창을 닫으면 트레이에서 유지됩니다. **Restore original**로 복원할 수 있습니다.

**0**은 원래 감마, **90**은 원래 감마 값의 10%를 요청합니다. 모니터 백라이트 밝기 10%를 뜻하지 않습니다. Windows x64와 Microsoft Edge WebView2 Runtime이 필요합니다.

## 빌드

```sh
cargo build --release --workspace
cargo test --workspace
node scripts/ui-smoke.cjs
```

결과: `target/release/gamma-dimmer-app.exe` (`ScreenBrightController.exe`로 이름 변경 가능). Rust/MSVC 및 Tauri의 Windows 빌드 환경이 필요하며, Node는 UI 테스트에만 사용합니다.

검증 범위와 남은 수동 확인 사항은 [VALIDATION.md](VALIDATION.md)에 있습니다. 실행 파일과 로컬 진단 자료는 소스 저장소에 포함하지 않습니다.
