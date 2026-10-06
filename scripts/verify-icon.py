"""Read-only checks: preserve supplied ICO; verify derived tray pixels and PE icon resources."""
from pathlib import Path
import ctypes, hashlib, json, struct, zlib
root = Path(__file__).resolve().parents[1]
icons = root / 'app/src-tauri/icons'
expected = '1cbe4d747d8ef3e26840a6b700b48e83a3440fa333fbf6c994114acea72f6398'
data = (icons / 'icon.ico').read_bytes()
assert hashlib.sha256(data).hexdigest() == expected, 'Supplied ICO changed'
assert data[:4] == b'\0\0\1\0'
entries = {}
for i in range(struct.unpack_from('<H', data, 4)[0]):
    w, h, _, _, _, _, size, offset = struct.unpack_from('<BBBBHHII', data, 6 + i * 16)
    entries[w or 256] = data[offset:offset + size]
assert (icons / 'icon.png').read_bytes() == entries[256]
assert (icons / 'tray.png').read_bytes() == entries[32]
# PNG decode using stdlib (8-bit RGBA only), no image regeneration or ICO writes.
png = entries[32]
assert png[:8] == b'\x89PNG\r\n\x1a\n'
pos = 8
idat = b''
while pos < len(png):
    length = struct.unpack_from('>I', png, pos)[0]
    kind = png[pos + 4:pos + 8]
    content = png[pos + 8:pos + 8 + length]
    if kind == b'IHDR':
        width, height, depth, color, _, _, interlace = struct.unpack('>IIBBBBB', content)
        assert (width, height, depth, color, interlace) == (32, 32, 8, 6, 0)
    if kind == b'IDAT':
        idat += content
    pos += length + 12
raw = zlib.decompress(idat)
stride = width * 4
previous = bytearray(stride)
pixels = bytearray()
for y in range(height):
    start = y * (stride + 1)
    filt = raw[start]
    row = bytearray(raw[start + 1:start + 1 + stride])
    for x in range(stride):
        a = row[x - 4] if x >= 4 else 0
        b = previous[x]
        c = previous[x - 4] if x >= 4 else 0
        if filt == 1:
            predictor = a
        elif filt == 2:
            predictor = b
        elif filt == 3:
            predictor = (a + b) // 2
        elif filt == 4:
            p = a + b - c
            distances = [abs(p - a), abs(p - b), abs(p - c)]
            predictor = [a, b, c][distances.index(min(distances))]
        else:
            assert filt == 0
            predictor = 0
        row[x] = (row[x] + predictor) & 255
    pixels.extend(row)
    previous = row
assert bytes(pixels) == (icons / 'tray.rgba').read_bytes(), 'Tray not from supplied icon'
# Open executable strictly as a resource data file: no app code/startup/display calls.
kernel = ctypes.WinDLL('kernel32', use_last_error=True)
kernel.LoadLibraryExW.argtypes = [ctypes.c_wchar_p, ctypes.c_void_p, ctypes.c_uint32]
kernel.LoadLibraryExW.restype = ctypes.c_void_p
kernel.FindResourceW.argtypes = [ctypes.c_void_p, ctypes.c_void_p, ctypes.c_void_p]
kernel.FindResourceW.restype = ctypes.c_void_p
kernel.LoadResource.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
kernel.LoadResource.restype = ctypes.c_void_p
kernel.LockResource.argtypes = [ctypes.c_void_p]
kernel.LockResource.restype = ctypes.c_void_p
kernel.SizeofResource.argtypes = [ctypes.c_void_p, ctypes.c_void_p]
kernel.SizeofResource.restype = ctypes.c_uint32
kernel.FreeLibrary.argtypes = [ctypes.c_void_p]
module = kernel.LoadLibraryExW(str(root / 'dist/ScreenBrightController-v0.5.exe'), None, 2)
assert module
resource_bytes = []
try:
    # Tauri / winres embeds the application's supplied ICO as group ID 32512 (IDI_APPLICATION).
    group = kernel.FindResourceW(module, ctypes.c_void_p(32512), ctypes.c_void_p(14))
    assert group, 'PE group icon missing'
    pointer = kernel.LockResource(kernel.LoadResource(module, group))
    group_data = ctypes.string_at(pointer, kernel.SizeofResource(module, group))
    count = struct.unpack_from('<H', group_data, 4)[0]
    for i in range(count):
        _, _, _, _, _, _, _, resource_id = struct.unpack_from('<BBBBHHIH', group_data, 6 + i * 14)
        res = kernel.FindResourceW(module, ctypes.c_void_p(resource_id), ctypes.c_void_p(3))
        assert res
        pointer = kernel.LockResource(kernel.LoadResource(module, res))
        resource_bytes.append(ctypes.string_at(pointer, kernel.SizeofResource(module, res)))
finally:
    kernel.FreeLibrary(module)
assert sorted(resource_bytes) == sorted(entries.values()), 'PE icon differs from supplied ICO'
version = ctypes.WinDLL('version', use_last_error=True)
version.GetFileVersionInfoSizeW.argtypes = [ctypes.c_wchar_p, ctypes.c_void_p]
version.GetFileVersionInfoW.argtypes = [ctypes.c_wchar_p, ctypes.c_uint32, ctypes.c_uint32, ctypes.c_void_p]
version.VerQueryValueW.argtypes = [ctypes.c_void_p, ctypes.c_wchar_p, ctypes.POINTER(ctypes.c_void_p), ctypes.POINTER(ctypes.c_uint)]
exe = str(root / 'dist/ScreenBrightController-v0.5.exe')
size = version.GetFileVersionInfoSizeW(exe, None)
assert size
buffer = ctypes.create_string_buffer(size)
assert version.GetFileVersionInfoW(exe, 0, size, buffer)
pointer = ctypes.c_void_p()
length = ctypes.c_uint()
assert version.VerQueryValueW(buffer, r'\VarFileInfo\Translation', ctypes.byref(pointer), ctypes.byref(length))
translation = (ctypes.c_ushort * 2).from_address(pointer.value)
base = '\\StringFileInfo\\%04x%04x\\' % (translation[0], translation[1])
metadata = {}
for key in ('ProductName', 'FileDescription', 'ProductVersion'):
    assert version.VerQueryValueW(buffer, base + key, ctypes.byref(pointer), ctypes.byref(length))
    metadata[key] = ctypes.wstring_at(pointer.value)
assert metadata['ProductName'] == metadata['FileDescription'] == 'Screen Bright Controller'
assert metadata['ProductVersion'] == '0.5.0'
result = {'metadata': metadata, 'supplied_ico_sha256': expected, 'ico_unchanged': True, 'pe_icon_entries_match': len(resource_bytes), 'window_png_matches_ico_256': True, 'tray_rgba_matches_ico_32': True, 'native_display_writes': 0}
(root / 'evidence/icon-v05.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
print(json.dumps(result, indent=2))
