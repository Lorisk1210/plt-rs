# Use font files from matplotlib's data directory by path: FontProperties(fname=...), text drawn with them, widths compared.
import os
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.font_manager import FontProperties, get_font

fonts_dir = os.path.join(matplotlib.get_data_path(), "fonts", "ttf")
names = ["DejaVuSans.ttf", "DejaVuSans-Bold.ttf", "DejaVuSerif-Italic.ttf", "DejaVuSansMono.ttf", "cmr10.ttf", "STIXGeneral.ttf"]
fig, ax = plt.subplots(figsize=(6, 4))
texts = []
for k, name in enumerate(names):
    fp = FontProperties(fname=os.path.join(fonts_dir, name), size=14)
    t = ax.text(0.02, 0.9 - k * 0.15, f"{name}: Wave 0123", fontproperties=fp, transform=ax.transAxes)
    texts.append(t)
fig.canvas.draw()
r = fig.canvas.get_renderer()
for name, t in zip(names, texts):
    fp = t.get_fontproperties()
    print(name, os.path.basename(fp.get_file()), fp.get_size_in_points(), round(t.get_window_extent(r).width, 2))
font = get_font(os.path.join(fonts_dir, "DejaVuSans.ttf"))
print(font.family_name, font.style_name, font.num_glyphs, font.units_per_EM)
print(font.postscript_name, font.num_faces, round(font.ascender / font.units_per_EM, 4))
fp = FontProperties(fname=os.path.join(fonts_dir, "DejaVuSans.ttf"))
print(fp.get_family(), fp.get_name())
w, h, d = r.get_text_width_height_descent("Wave", fp, ismath=False)
print(round(w, 3), round(h, 3), round(d, 3))
fig.savefig("fonts_by_path.png")
bold = FontProperties(fname=os.path.join(fonts_dir, "DejaVuSans-Bold.ttf"))
wb, _, _ = r.get_text_width_height_descent("Wave", bold, ismath=False)
print(round(wb - w, 3), bold.get_weight(), os.path.basename(bold.get_file()))
