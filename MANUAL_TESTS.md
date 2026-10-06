# Manual acceptance — v0.5

Not executed by development tools. Use only with deliberate user authorization. Restore and quit the older dimmer first; do not run two gamma controllers together. Turn off HDR / conflicting color tools. Start with a mild value and keep header ↺ / tray **Restore original** reachable.

## Windows login startup (not performed; explicit opt-in required)

- First launch: gear Settings is collapsed; Start with Windows is OFF unless this exact executable was previously registered. Opening Settings and focusing the app only query registration; never create it.
- Put the executable at a stable path containing spaces. Explicitly enable. Wait for verified checked state; inspect the per-user Run value read-only: quoted current executable plus only `--autostart`. No administrator prompt. Settings must not dim or restore displays.
- Sign out/in only when ready: tray icon present, main and popup initially hidden, no gamma/dimming/brightness restoration. Open app from the tray and verify all controls start at 0 and originals remain untouched until a deliberate dimming gesture.
- If already running, invoke a second `--autostart`: no main-window activation and no dimming. A normal second instance still opens main. Keep only one gamma controller active.
- Explicitly disable and verify unchecked state/value removal. At next login, app must not run. Repeated disable is harmless.
- Move/rename only after normal Restore and quit. Reopen: stale old location is reported, not incorrectly checked; enable again to update the path. Leave the executable in its final location afterward.
- Compare Windows Startup Apps/Task Manager policy separately; this app does not modify its external disabled/approval switches. Verify behavior when those policies allow startup.
- Permissions/readback failure and mismatch are injected automated tests, not real registry tampering. On an actual error, expect verified rollback/current state; unknown state disables the checkbox and reopening Settings retries the read. Never accept a checked box alone as proof of real login execution.

## Existing controls

- Startup, reopening, second instance and status: no dimming until an actual gesture.
- Main/tray: clean Display 1 / Display 2 labels and shared values. Popup remains open after clicking another window; X, Escape, tray toggle and Open app hide it deliberately.
- Slider drag/click uses integer steps of 1. Slider arrows / wheel add or subtract 5 from the current value (23→28), Home=0, End=90. Number typing / spinner / arrows use steps of 1; invalid fractions / out-of-range values do not apply. No modal confirmation.
- Rapid drag: controls remain responsive; final released value wins. Restore during pending input must not be followed by late re-dimming, including the other window.
- Linked / independent / include controls: verify intended displays; zero and exclusion restore their originals.
- Boundary investigation, one intentional request at a time: 44, 45, 46, 49, 50, 51, 55, 90. Record requested value, Win32 API result, readback match and visible effect separately. Stop at the first failure; Restore before another attempt. Do not sweep automatically, remap values or change gamma-related registry settings. Mock tests do not establish the Windows/driver cutoff.
- If an update is ignored but readback still equals the last verified ramp, that ramp should remain protected without repeated SET calls. Otherwise expect an honest error and best-effort restoration; requested UI values alone are not proof of effective gamma.
- Check actual hardware cursor, topmost / fullscreen windows and each display. Screenshots cannot establish gamma's visible effect.
- Hide main for longer than 15 seconds; continuous dimming remains protected by the native worker. Header Restore and tray Restore / Restore and quit return saved originals. Restoration failure must cancel Quit and permit retry.
- Verify native WebView2 focus, popup work-area placement / mixed DPI and hotplug separately. Do not kill both application and watchdog to test recovery.
