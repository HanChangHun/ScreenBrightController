"""Popup integration contracts; no native GUI, gamma or registration writes."""
import json
from pathlib import Path
import unittest
import tomllib

ROOT = Path(__file__).resolve().parents[1]


class PopupGeometryTests(unittest.TestCase):
    def test_all_package_versions_are_052_without_identity_changes(self):
        for file in ('Cargo.toml', 'app/src-tauri/Cargo.toml'):
            self.assertEqual(tomllib.loads((ROOT / file).read_text())['package']['version'], '0.5.2')
        lock = tomllib.loads((ROOT / 'Cargo.lock').read_text())
        local = [p for p in lock['package'] if p['name'] in ('screen-bright-controller', 'screen-bright-controller-tray')]
        self.assertEqual([p['version'] for p in local], ['0.5.2', '0.5.2'])
        for file in ('app/package.json', 'app/package-lock.json', 'app/src-tauri/tauri.conf.json'):
            self.assertEqual(json.loads((ROOT / file).read_text())['version'], '0.5.2')
        self.assertEqual(json.loads((ROOT / 'app/package-lock.json').read_text())['packages']['']['version'], '0.5.2')

    def test_native_reopen_reuses_actual_geometry_and_handles_screen_changes(self):
        native = (ROOT / 'app/src-tauri/src/main.rs').read_text(encoding='utf-8')
        show = native.split('fn show_popup(', 1)[1].split('fn popup_should_hide', 1)[0]
        self.assertIn('prepare_popup(app, Some(position), false)', show)
        self.assertNotIn('popup_bounds(', show, 'First-show anchoring must not reset every reopen')
        self.assertIn('w.inner_size()', native)
        self.assertIn('w.outer_position()', native)
        self.assertIn('popup_reopen_bounds(&areas, current)', native)
        self.assertIn('ScaleFactorChanged', native)
        self.assertIn('prepare_popup(&handle, None, false)', native, 'Reconcile topology while visible or hidden')
        geometry = native.split('fn prepare_popup(', 1)[1].split('fn show_popup(', 1)[0]
        for operation in ('UiSession', 'live_control', 'restore_app', 'emit('):
            self.assertNotIn(operation, geometry, 'Geometry must not write gamma or refresh display controls')
        self.assertIn('popup_should_hide(event)', native)
        self.assertLess(geometry.index('w.set_position('), geometry.index('w.set_min_size(Some('),
                        'Move to target DPI before binding physical min/max sizes')
        self.assertIn('let actual_size = w.inner_size()', geometry,
                      'Read post-move DPI size before deciding whether resize is necessary')

    def test_selected_monitor_is_checked_before_skipping_native_move_reconciliation(self):
        native = (ROOT / 'app/src-tauri/src/main.rs').read_text(encoding='utf-8')
        events = native.split('.on_window_event(', 1)[1].split('.build(tauri::generate_context!', 1)[0]
        self.assertIn('tauri::WindowEvent::Moved(_)', events,
                      'Same-DPI monitor moves do not generate ScaleFactorChanged')
        self.assertIn('prepare_popup(&handle, None, false)', native,
                      'Keep the periodic fallback for missed movement events')
        geometry = native.split('fn prepare_popup(', 1)[1].split('fn show_popup(', 1)[0]
        self.assertLess(geometry.index('w.outer_position()'), geometry.index('needs_reconcile('))
        self.assertLess(geometry.index('popup_reopen_bounds(&areas, current)'),
                        geometry.index('needs_reconcile('),
                        'Unchanged topology cannot skip selection of the current work area')

    def test_native_setter_events_are_guarded_without_holding_geometry_lock(self):
        native = (ROOT / 'app/src-tauri/src/main.rs').read_text(encoding='utf-8')
        geometry = native.split('fn prepare_popup(', 1)[1].split('fn show_popup(', 1)[0]
        self.assertIn('guard.reconciling', geometry,
                      'Window setters may synchronously emit Moved / DPI events')
        self.assertLess(geometry.index('guard.reconciling = true'), geometry.index('w.set_min_size('))
        self.assertIn('let result = (||', geometry,
                      'Setter errors must leave the in-progress state resettable')
        reset = geometry.index('guard.reconciling = false')
        self.assertGreater(reset, geometry.index('w.set_size('))
        self.assertGreater(geometry.index('guard.selected_area ='), reset,
                           'Only successful reconciliation advances the selected area')
        self.assertIn('if result.is_ok()', geometry)
        self.assertNotIn('.lock()', geometry.split('let result = (||', 1)[1].split('})();', 1)[0],
                         'Never hold/acquire the geometry mutex around native setters')

    def test_header_and_all_native_resize_affordances_are_loaded(self):
        html = (ROOT / 'app/ui/index.html').read_text(encoding='utf-8')
        self.assertIn('<script src="popup-geometry.js" defer></script>', html)
        for direction in ('North', 'NorthEast', 'East', 'SouthEast', 'South',
                          'SouthWest', 'West', 'NorthWest'):
            self.assertEqual(html.count(f'data-resize="{direction}"'), 1)
        self.assertNotIn('data-tauri-drag-region', html, 'Do not enable ungated global dragging')
        css = (ROOT / 'app/ui/style.css').read_text(encoding='utf-8')
        self.assertIn('cursor:move', css)
        for cursor in ('ew-resize', 'ns-resize', 'nesw-resize', 'nwse-resize'):
            self.assertIn(cursor, css)
        gestures = (ROOT / 'app/ui/popup-geometry.js').read_text(encoding='utf-8')
        for command in ('live_control', 'restore', 'set_autostart', 'set_size', 'set_position'):
            self.assertNotIn(command, gestures)
        self.assertIn('button, input, select, textarea, a, label', gestures)

    def test_native_resize_uses_only_required_local_popup_permissions(self):
        config = json.loads((ROOT / 'app/src-tauri/tauri.conf.json').read_text(encoding='utf-8'))
        popup = config['app']['windows'][0]
        self.assertTrue(popup['resizable'])
        self.assertEqual((popup['minWidth'], popup['minHeight']), (430, 260))
        capability = json.loads((ROOT / 'app/src-tauri/capabilities/main.json').read_text(encoding='utf-8'))
        self.assertEqual(capability['windows'], ['popup'])
        self.assertNotIn('remote', capability)
        self.assertEqual(set(capability['permissions']), {
            'core:event:allow-listen', 'core:event:allow-unlisten',
            'core:window:allow-start-dragging', 'core:window:allow-start-resize-dragging',
        })


if __name__ == '__main__':
    unittest.main()
