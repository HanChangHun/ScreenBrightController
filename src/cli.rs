use crate::{
    controller::{Controller, Driver, Mock},
    native::Native,
    watchdog::{Link, Request},
};
use serde_json::json;
fn print(value: serde_json::Value) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?
    );
    Ok(())
}
pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
  Some("--mock-parent-wait")=>{use std::io::Write;println!("READY");std::io::stdout().flush().map_err(|e|e.to_string())?;std::thread::sleep(std::time::Duration::from_secs(60));Ok(())},
  Some("--watchdog")=>crate::watchdog::serve(false),Some("--watchdog-mock")=>crate::watchdog::serve(true),
  Some("--startup-check")=>{
   let mut controller=Controller::new(crate::watchdog::Real::new()?)?;
   let status=controller.driver.status()?;
   if !status.armed.is_empty(){return Err("unexpected armed state at startup".into())}
   controller.restore_all()?;
   let final_state=controller.driver.link.disconnect_and_wait()?;
   print(json!({"mode":"read-only-startup","native_display_writes":0,"monitor_count":controller.monitors.len(),"armed":final_state.armed,"restore_errors":final_state.restore_errors}))
  },
  Some("--diagnose")=>print(json!({"mode":"read-only","native_display_writes":0,"hdr_status":"not detected; legacy gamma undefined under HDR","visible_effect_verified":false,"monitors":Native.snapshot()?})),
  Some("--self-test")|Some("--mock")=>self_test(),
  _=>Err("Use --diagnose (read-only), --self-test / --mock (memory-only), or launch gamma-dimmer-app.exe for UI. No CLI applies real dimming.".into())
 }
}
fn self_test() -> Result<(), String> {
    let mut c = Controller::new(Mock::default())?;
    if c.driver.writes != 0 {
        return Err("startup wrote gamma".into());
    }
    if c.preview(&["mock-1".into()], 50, 15, false).is_ok() {
        return Err("consent guard failed".into());
    }
    let outcomes = c.preview(&["mock-1".into(), "mock-2".into()], 75, 15, true)?;
    c.restore_all()?;
    if c.driver.current["mock-1"][0][0] != 40000 || c.driver.current["mock-2"][0][0] != 50000 {
        return Err("saved-original restore failed".into());
    }
    let mut watchdog = Link::spawn(true)?;
    let armed = watchdog.request(Request::Arm {
        id: "mock-1".into(),
        seconds: 1,
    })?;
    if armed.mock_values.as_ref().unwrap()["mock-1"] != 100 {
        return Err("mock watchdog did not simulate change".into());
    }
    std::thread::sleep(std::time::Duration::from_millis(1300));
    let timed = watchdog.request(Request::Status)?;
    if !timed.armed.is_empty() || timed.mock_values.as_ref().unwrap()["mock-1"] != 40000 {
        return Err("independent timeout restore failed".into());
    }
    watchdog.request(Request::Arm {
        id: "mock-2".into(),
        seconds: 30,
    })?;
    let disconnected = watchdog.disconnect_and_wait()?;
    if !disconnected.armed.is_empty()
        || disconnected.mock_values.as_ref().unwrap()["mock-2"] != 50000
    {
        return Err("pipe EOF restore failed".into());
    }
    print(
        json!({"mode":"mock","native_display_writes":0,"mock_controller_writes":c.driver.writes,"monitor_count":c.monitors.len(),"outcomes":outcomes,"watchdog_timeout_restored":true,"watchdog_disconnect_restored":true,"visible_effect_verified":false}),
    )
}
