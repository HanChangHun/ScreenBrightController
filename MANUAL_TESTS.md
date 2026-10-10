# Manual acceptance — v0.6.0

Not executed by development tools. Use only with deliberate user authorization. Restore and quit the older controller first; do not run two gamma controllers together. Turn off HDR / conflicting color tools. Start with a mild value and keep header ↺ / tray **Restore original** reachable.

## Windows login startup (not performed; explicit opt-in required)

- First launch: only the tray icon is shown. Left-click it: popup gear Settings is collapsed; Start with Windows is OFF unless this exact executable was previously registered. Opening Settings and focusing the popup only query registration; never create it.
- Put the executable at a stable path containing spaces. Explicitly enable. Wait for verified checked state; inspect the per-user Run value read-only: quoted current executable plus only `--autostart`. No administrator prompt. Settings must not dim or restore displays.
- Sign out/in only when ready: tray icon present, sole popup hidden and no separate main window, no gamma/dimming/brightness restoration. Left-click the tray and verify all controls start at 0 and originals remain untouched until a deliberate dimming gesture.
- If already running, invoke a second normal launch or `--autostart`: no popup activation, new window or dimming. Only tray clicking opens the controls. Keep only one gamma controller active.
- Explicitly disable and verify unchecked state/value removal. At next login, app must not run. Repeated disable is harmless.
- Move/rename only after normal Restore and quit. Reopen: stale old location is reported, not incorrectly checked; enable again to update the path. Leave the executable in its final location afterward.
- Compare Windows Startup Apps/Task Manager policy separately; this app does not modify its external disabled/approval switches. Verify behavior when those policies allow startup.
- Permissions/readback failure and mismatch are injected automated tests, not real registry tampering. On an actual error, expect the read-back current state with an error; unknown state disables the checkbox and reopening Settings retries the read. Never accept a checked box alone as proof of real login execution.

## Popup geometry (native acceptance not performed)

- First tray open: 430×340 logical popup placed within the clicked monitor's work area. Drag title/header blank space with the left mouse button; Settings, Restore, − and all inputs must not move the window. Right/middle click must not start the custom drag. A double-click's second press (`detail === 2`) is suppressed; its first primary press can still initiate dragging.
- Drag each of four borders and four corners; cursor hints and lower-right grip are present. Native resizing respects the 430×260 logical minimum and current work area; settings/recovery/many display rows remain scrollable with readable controls.
- Move and resize, then hide with −, Escape and tray toggle separately. Reopen from either monitor's tray: retain the adjusted geometry, not a fresh tray anchor/default size. Restart is intentionally **not** persisted.
- Test moves between existing same-DPI monitors of different work-area dimensions, mixed DPI, negative monitor origins, work-area/taskbar changes and monitor removal while visible and while hidden. Rebind native min/max constraints when the selected work area changes; ordinary same-area dragging must not keep snapping the window back into bounds. Preserve/clamp existing geometry to the best remaining work area; native move/DPI events reconcile on the main thread, with a two-second periodic fallback and reopen check. Avoid native default/minimum inflation when first opening on a different-DPI monitor.
- Keep dimming controls untouched; compare read-only native ramps and startup registration before/after all movement/resizing/hiding. Never launch two real controllers together or kill the live watchdog for this acceptance.

## Dimming and recovery

- The sole tray popup omits Open app, the eyebrow/version badge, section heading/range caption and repeated row notes. Keep display names, checkboxes, endpoint numbers, gear Settings and Restore.
- On an actual application failure, all dimming inputs in the tray popup become disabled. Only one concise recovery notice is shown; requested numbers are marked unconfirmed. Expand Details to distinguish API acceptance from readback match/error. Do not infer a native cutoff from the requested value.
- Restore is an explicit action, not an automatic retry. On success the notice clears and controls return to zero; only a new deliberate input can dim again. If restoration fails, keep the app open and retry Restore. A status transport failure must also prevent new requests.

- Startup, reopening, second instance and status: no dimming until an actual gesture.
- Tray popup: clean Display 1 / Display 2 labels. It remains open after clicking another window; −, Escape and the tray icon toggle hide it deliberately. No Open app button/menu or taskbar main window remains.
- Slider drag/click uses integer steps of 1. Slider arrows / wheel add or subtract 5 from the current value (23→28), Home=0, End=90. Number typing / spinner / arrows use steps of 1; invalid fractions / out-of-range values do not apply. No modal confirmation.
- Rapid drag: controls remain responsive; final released value wins. Restore during pending input must not be followed by late re-dimming, including a request already in flight.
- Linked / independent / include controls: verify intended displays; zero and exclusion restore their originals.
- Boundary investigation, one intentional request at a time: 44, 45, 46, 49, 50, 51, 55, 90. Record requested value, Win32 API result, readback match and visible effect separately. Stop at the first failure; Restore before another attempt. Do not sweep automatically, remap values or change gamma-related registry settings. Mock tests do not establish the Windows/driver cutoff.
- If an update is ignored but readback still equals the last verified ramp, that ramp should remain protected without repeated SET calls. Otherwise expect an honest error and best-effort restoration; requested UI values alone are not proof of effective gamma.
- Check actual hardware cursor, topmost / fullscreen windows and each display. Screenshots cannot establish gamma's visible effect.
- Hide the popup for longer than 15 seconds; continuous dimming remains protected by the native worker. Header Restore and tray Restore / Restore and quit return saved originals. Restoration failure must cancel Quit and permit retry.
- Verify native WebView2 focus, popup work-area placement / mixed DPI and hotplug separately. Do not kill both application and watchdog to test recovery.
