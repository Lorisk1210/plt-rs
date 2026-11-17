# TextPath of strings with kerning pairs, multiline text, and usetex off; widths and vertex counts across sizes and fonts.
import numpy as np
from matplotlib.font_manager import FontProperties
from matplotlib.textpath import TextPath, TextToPath

strings = ["AVATAR", "To be", "WAVE", "fi fl", "1234567890", "Hello, world!"]
for family in ("DejaVu Sans", "DejaVu Serif", "DejaVu Sans Mono"):
    fp = FontProperties(family=family)
    for s in strings:
        tp = TextPath((0, 0), s, size=10, prop=fp)
        ext = tp.get_extents()
        print(family, repr(s), len(tp.vertices), round(ext.width, 3), round(ext.height, 3))
ttp = TextToPath()
fp = FontProperties(family="DejaVu Sans", size=12)
w, h, d = ttp.get_text_width_height_descent("AVATAR", fp, ismath=False)
print(round(w, 4), round(h, 4), round(d, 4))
verts, codes = ttp.get_text_path(fp, "AV")
print(len(verts), len(codes), sorted(set(int(c) for c in codes)))
glyph_info, glyph_map, rects = ttp.get_glyphs_with_font(ttp._get_font(fp), "AVA")
print(len(glyph_info), len(glyph_map), rects)
multi = TextPath((0, 0), "two\nlines", size=10)
print(len(multi.vertices), round(multi.get_extents().height, 3))
moved = TextPath((5, 7), "x", size=20)
print(np.round(moved.get_extents().bounds, 3).tolist())
print(TextPath((0, 0), ".", size=10).vertices.shape)
tp = TextPath((0, 0), "abc", size=10)
print(tp.get_size(), len(tp.vertices))
tp.set_size(20)
print(tp.get_size(), round(tp.get_extents().width, 3))
mono = [round(TextPath((0, 0), c * 4, size=10, prop=FontProperties(family="DejaVu Sans Mono")).get_extents().x1, 3) for c in "il"]
print(mono)
