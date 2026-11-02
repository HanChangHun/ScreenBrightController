use screen_bright_controller::controller::*;
#[cfg(windows)]
#[test]
fn log_files_live_in_the_tauri_app_data_directory() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../app/src-tauri/tauri.conf.json")).unwrap();
    assert_eq!(
        config["identifier"],
        screen_bright_controller::log::IDENTIFIER
    );
    let dir = screen_bright_controller::log::dir().unwrap();
    assert!(
        dir.ends_with("io.github.screenbrightcontroller\\logs"),
        "{dir:?}"
    );
}
#[cfg(windows)]
#[test]
fn log_lines_stay_single_and_rotate_at_the_size_limit() {
    use screen_bright_controller::log::{append, line};
    let one = line("restore failed\r\nforged line");
    assert_eq!(one.matches('\n').count(), 1, "{one}");
    assert!(
        !one.contains('\r') && one.ends_with("restore failed  forged line\n"),
        "{one}"
    );
    assert!(one.contains(&format!("[{}]", std::process::id())), "{one}");
    let dir = std::env::temp_dir().join(format!("sbc-log-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let path = dir.join("app.log");
    append(&path, "first\n", 10);
    append(&path, "second\n", 10);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "first\nsecond\n");
    append(&path, "third\n", 10);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "third\n");
    assert_eq!(
        std::fs::read_to_string(dir.join("app.log.1")).unwrap(),
        "first\nsecond\n"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
#[test]
fn update_release_accepts_only_new_stable_exact_repository_assets() {
    use screen_bright_controller::update::Release;
    let raw = |tag: &str| {
        serde_json::json!({
            "tag_name":tag,"draft":false,"prerelease":false,"body":"Notes",
            "assets":[{"name":"ScreenBrightController_0.6.3_x64-setup.exe","size":1000,"browser_download_url":"https://github.com/HanChangHun/ScreenBrightController/releases/download/v0.6.3/ScreenBrightController_0.6.3_x64-setup.exe"},
            {"name":"SHA256SUMS.txt","size":110,"browser_download_url":"https://github.com/HanChangHun/ScreenBrightController/releases/download/v0.6.3/SHA256SUMS.txt"}]
        })
    };
    let good = raw("v0.6.3");
    assert_eq!(
        Release::parse(&good.to_string(), "0.6.2")
            .unwrap()
            .unwrap()
            .version,
        "0.6.3"
    );
    assert!(Release::parse(&good.to_string(), "0.6.3")
        .unwrap()
        .is_none());
    assert!(Release::parse(&good.to_string(), "0.7.0")
        .unwrap()
        .is_none());
    for tag in ["0.6.3", "v0.6.3-beta.1", "v00.6.3", "v0.6.3/../x"] {
        assert!(Release::parse(&raw(tag).to_string(), "0.6.2").is_err());
    }
    for key in ["draft", "prerelease"] {
        let mut bad = good.clone();
        bad[key] = true.into();
        assert!(Release::parse(&bad.to_string(), "0.6.2").is_err());
    }
    let mut bad = good.clone();
    bad["assets"][0]["browser_download_url"] = "https://evil.example/setup.exe".into();
    assert!(Release::parse(&bad.to_string(), "0.6.2").is_err());
    let mut bad = good.clone();
    bad["assets"][0]["size"] = 0.into();
    assert!(Release::parse(&bad.to_string(), "0.6.2").is_err());
    let mut bad = good.clone();
    bad["assets"][1]["name"] = "../SHA256SUMS.txt".into();
    assert!(Release::parse(&bad.to_string(), "0.6.2").is_err());
    let mut bad = good.clone();
    bad["assets"]
        .as_array_mut()
        .unwrap()
        .push(good["assets"][0].clone());
    assert!(Release::parse(&bad.to_string(), "0.6.2").is_err());
}
#[test]
fn update_checksum_requires_exact_unique_installer_and_matching_bytes() {
    use screen_bright_controller::update::{expected_checksum, verify_bytes};
    let hash = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let name = "ScreenBrightController_0.6.3_x64-setup.exe";
    let sums = format!("{hash}  {name}\n");
    assert_eq!(expected_checksum(&sums, name).unwrap(), hash);
    assert!(expected_checksum(&(sums.clone() + &sums), name).is_err());
    assert!(expected_checksum(&sums, "other.exe").is_err());
    assert!(expected_checksum(&sums.replace(hash, "bad"), name).is_err());
    assert!(verify_bytes(b"abc", 3, hash).is_ok());
    assert!(verify_bytes(b"abd", 3, hash).is_err());
    assert!(verify_bytes(b"abc", 4, hash).is_err());
}

#[test]
fn restoration_failure_preserves_original_and_allows_retry() {
    let mut c = Controller::new(Mock {
        fail_restore: true,
        ..Default::default()
    })
    .unwrap();
    c.apply_continuous(&["mock-1".into()], 50).unwrap();
    assert!(c.restore_all().is_err());
    assert!(c.changed.contains("mock-1"));
    c.driver.fail_restore = false;
    c.restore_all().unwrap();
    assert_eq!(c.driver.current["mock-1"][0][0], 40000);
}
#[test]
fn watchdog_failure_prevents_display_write() {
    let mut c = Controller::new(Mock {
        fail_arm: true,
        ..Default::default()
    })
    .unwrap();
    assert!(c.apply_continuous(&["mock-1".into()], 50).is_err());
    assert_eq!(c.driver.writes, 0);
    assert!(c.changed.is_empty());
}
#[test]
fn api_success_is_not_readback_or_visible_effect() {
    let mut c = Controller::new(Mock {
        ignored_set: true,
        ..Default::default()
    })
    .unwrap();
    let out = c.apply_continuous(&["mock-1".into()], 50).unwrap();
    assert!(out[0].api_success);
    assert_eq!(out[0].readback_matches, Some(false));
    c.restore_all().unwrap();
}
#[test]
fn failed_api_still_restores_potentially_changed_display() {
    let mut c = Controller::new(Mock {
        fail_set: true,
        ..Default::default()
    })
    .unwrap();
    let out = c.apply_continuous(&["mock-1".into()], 50).unwrap();
    assert!(!out[0].api_success);
    assert!(c.changed.contains("mock-1"));
    c.restore_all().unwrap();
    assert_eq!(c.driver.current["mock-1"][0][0], 40000);
}
#[test]
fn all_monitors_apply_and_restore() {
    let mut c = Controller::new(Mock::default()).unwrap();
    c.apply_continuous(&["mock-1".into(), "mock-2".into()], 50)
        .unwrap();
    c.restore_all().unwrap();
    assert_eq!(c.driver.restored.len(), 2);
}
