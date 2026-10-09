"""Reproducible transparent diagonal-monitor icon assets."""
from pathlib import Path
from PIL import Image, ImageDraw
import shutil

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'app/src-tauri/icons'
OUT.mkdir(parents=True, exist_ok=True)
SIZES = [16,24,32,48,64,128,256]

def render(size):
    factor = 4
    canvas = Image.new('RGBA', (256*factor,256*factor))
    d = ImageDraw.Draw(canvas)
    def box(coords): return tuple(int(v*factor) for v in coords)
    # Neutral dual-contrast frame; no enclosing tile or glass highlight.
    d.rounded_rectangle(box((16,28,240,192)), radius=18*factor, fill='#8D959D')
    d.rounded_rectangle(box((22,34,234,186)), radius=13*factor, fill='#262B31')
    mask = Image.new('L',canvas.size)
    ImageDraw.Draw(mask).rounded_rectangle(box((34,46,222,174)),radius=7*factor,fill=255)
    screen = Image.new('RGBA',canvas.size,'#3B424B')
    ImageDraw.Draw(screen).polygon([box((34,46)),box((222,46)),box((34,174))],fill='#F4EEDC')
    canvas.paste(screen,(0,0),mask)
    d = ImageDraw.Draw(canvas)
    d.rounded_rectangle(box((115,190,141,216)),radius=3*factor,fill='#8D959D')
    d.rounded_rectangle(box((80,211,176,225)),radius=7*factor,fill='#8D959D')
    return canvas.resize((size,size),Image.Resampling.LANCZOS)

old = OUT / 'icon.ico'
if old.exists() and not (OUT / 'icon-before-diagonal.ico').exists():
    shutil.copy2(old,OUT / 'icon-before-diagonal.ico')
master = render(256)
master.save(OUT / 'screen-bright-controller.png')
master.save(OUT / 'icon.png')
master.save(OUT / 'icon.ico',sizes=[(s,s) for s in SIZES],append_images=[render(s) for s in SIZES[:-1]])
for size in SIZES:
    render(size).save(OUT / f'screen-bright-controller-{size}.png')
tray = render(32)
tray.save(OUT / 'tray.png')
(OUT / 'tray.rgba').write_bytes(tray.tobytes())
svg = '''<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256"><defs><clipPath id="screen"><rect x="34" y="46" width="188" height="128" rx="7"/></clipPath></defs><rect x="16" y="28" width="224" height="164" rx="18" fill="#8D959D"/><rect x="22" y="34" width="212" height="152" rx="13" fill="#262B31"/><g clip-path="url(#screen)"><rect x="34" y="46" width="188" height="128" fill="#3B424B"/><path d="M34 46H222L34 174Z" fill="#F4EEDC"/></g><rect x="115" y="190" width="26" height="26" rx="3" fill="#8D959D"/><rect x="80" y="211" width="96" height="14" rx="7" fill="#8D959D"/></svg>'''
(OUT / 'screen-bright-controller.svg').write_text(svg,encoding='utf-8')
# Visual proof: actual exported sizes on both taskbar tones.
board = Image.new('RGB',(680,270),'#181B20')
d = ImageDraw.Draw(board)
d.rectangle((340,0,680,270),fill='#F2F2F0')
for xbase in (0,340):
    board.paste(master,(xbase+42,2),master)
    for x,size in [(12,16),(42,24),(84,32)]:
        im=render(size);board.paste(im,(xbase+x,235),im)
board.save(OUT / 'icon-preview.png')
print('Generated SVG, PNG, multi-resolution ICO and tray RGBA:',OUT)
