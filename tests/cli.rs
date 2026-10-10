#[test]
fn actual_startup_and_exit_path_are_read_only() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_screen-bright-controller-cli"))
        .arg("--startup-check")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["mode"], "read-only-startup");
    assert_eq!(result["armed"], serde_json::json!([]));
    assert_eq!(result["native_display_writes"], 0);
}
#[test]
fn process_death_restores_with_pipe_still_open() {
    use std::io::{BufRead, BufReader, Write};
    let exe = env!("CARGO_BIN_EXE_screen-bright-controller-cli");
    let mut parent = std::process::Command::new(exe)
        .arg("--mock-parent-wait")
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut ready = String::new();
    BufReader::new(parent.stdout.take().unwrap())
        .read_line(&mut ready)
        .unwrap();
    assert_eq!(ready.trim(), "READY");
    let mut guard = std::process::Command::new(exe)
        .arg("--watchdog-mock")
        .env(
            "SCREEN_BRIGHT_CONTROLLER_WATCHDOG_PARENT",
            parent.id().to_string(),
        )
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = guard.stdin.take().unwrap();
    let mut output = BufReader::new(guard.stdout.take().unwrap());
    let mut request = |v: serde_json::Value| {
        writeln!(input, "{v}").unwrap();
        input.flush().unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        serde_json::from_str::<serde_json::Value>(&line).unwrap()
    };
    assert_eq!(request(serde_json::json!({"command":"Hello"}))["ok"], true);
    assert_eq!(
        request(serde_json::json!({"command":"Continuous","id":"mock-1","lease":10}))
            ["mock_values"]["mock-1"],
        100
    );
    parent.kill().unwrap();
    parent.wait().unwrap();
    // Keep input alive: only the Windows process handle can trigger this restoration.
    let mut final_line = String::new();
    output.read_line(&mut final_line).unwrap();
    let result: serde_json::Value = serde_json::from_str(&final_line).unwrap();
    assert_eq!(result["mock_values"]["mock-1"], 40000);
    assert_eq!(result["armed"], serde_json::json!([]));
    drop(input);
    assert!(guard.wait().unwrap().success());
}
#[test]
fn real_cli_mock_smoke_uses_independent_watchdog_without_display_writes() {
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_screen-bright-controller-cli"))
        .arg("--self-test")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["native_display_writes"], 0);
    assert_eq!(report["watchdog_disconnect_restored"], true);
}
