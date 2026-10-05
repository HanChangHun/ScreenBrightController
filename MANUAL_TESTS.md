# Manual acceptance — v0.4

Not executed by development tools. Use only with deliberate user authorization. Restore and quit the older dimmer first; do not run two gamma controllers together. Turn off HDR / conflicting color tools. Start with a mild value and keep header ↺ / tray **Restore original** reachable.

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
