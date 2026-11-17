# Read a TrueType font through matplotlib.ft2font: char maps, glyph names, metrics, kerning pairs and a rendered bitmap.
import os
import numpy as np
import matplotlib
from matplotlib import ft2font
from matplotlib.font_manager import get_font

path = os.path.join(matplotlib.get_data_path(), "fonts", "ttf", "DejaVuSans.ttf")
font = get_font(path)
font.set_size(12, 72)
cmap = font.get_charmap()
print(len(cmap) > 3000, cmap[ord("A")], font.get_char_index(ord("A")))
for ch in "AVWa.":
    g = font.load_char(ord(ch), flags=ft2font.LoadFlags.NO_HINTING)
    print(ch, font.get_glyph_name(font.get_char_index(ord(ch))), g.width, g.height,
          g.horiAdvance, g.linearHoriAdvance, g.bbox)
pairs = [("A", "V"), ("V", "A"), ("T", "o"), ("a", "b"), ("L", "T")]
for a, b in pairs:
    ia, ib = font.get_char_index(ord(a)), font.get_char_index(ord(b))
    print(a + b, font.get_kerning(ia, ib, ft2font.Kerning.DEFAULT), font.get_kerning(ia, ib, ft2font.Kerning.UNFITTED))
font.set_text("AVATAR", 0.0, flags=ft2font.LoadFlags.NO_HINTING)
print(font.get_width_height(), font.get_descent(), font.get_bitmap_offset())
font.draw_glyphs_to_bitmap(antialiased=True)
img = font.get_image()
print(img.shape, int(img.max()), int((img > 0).sum()))
print(font.get_sfnt_table("head")["unitsPerEm"], sorted(font.get_sfnt_table("hhea"))[:4])
print(ft2font.FaceFlags.SCALABLE in font.face_flags, ft2font.FaceFlags.KERNING in font.face_flags)
verts, codes = font.get_path()
print(verts.shape[1], len(verts) == len(codes), len(verts) > 0)
print(font.style_flags, font.bbox, font.underline_position, font.underline_thickness)
print(ft2font.__freetype_version__ != "", ft2font.LoadFlags.DEFAULT.value)
