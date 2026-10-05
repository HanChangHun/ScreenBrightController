"""Generate original 32x32 Windows icon with stdlib only; build helper, not runtime."""
from pathlib import Path
import struct
root = Path(__file__).resolve().parents[1]
size = 32
pixels = bytearray()
for y in range(size):
    for x in range(size):
        color = (192, 208, 103, 255) if 6 <= x < 26 and 6 <= y < 26 else (37, 27, 18, 255)
        pixels.extend(color)
dib = struct.pack('<IiiHHIIiiII', 40, size, size * 2, 1, 32, 0, len(pixels), 0, 0, 0, 0)
dib += pixels + bytes(((size + 31) // 32) * 4 * size)
ico = struct.pack('<HHH', 0, 1, 1) + struct.pack('<BBBBHHII', size, size, 0, 0, 1, 32, len(dib), 22) + dib
path = root / 'app' / 'src-tauri' / 'icons' / 'icon.ico'
path.parent.mkdir(parents=True, exist_ok=True)
path.write_bytes(ico)
print(f'{path}: valid ICO header, {len(ico)} bytes')
