# Use TextPath outlines as plain geometry: area by shoelace on polygons, contains_points against a grid, and a clip path.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.font_manager import FontProperties
from matplotlib.path import Path
from matplotlib.patches import PathPatch
from matplotlib.textpath import TextPath


def polygon_area(poly):
    x, y = poly[:, 0], poly[:, 1]
    return 0.5 * float(np.dot(x, np.roll(y, 1)) - np.dot(y, np.roll(x, 1)))


fp = FontProperties(family="DejaVu Sans", weight="bold")
tp = TextPath((0, 0), "OHA", size=40, prop=fp)
polys = tp.to_polygons()
areas = [round(polygon_area(q), 1) for q in polys]
print(len(tp.vertices), len(polys), areas)
ext = tp.get_extents()
print([round(v, 2) for v in ext.bounds])
gx, gy = np.meshgrid(np.linspace(ext.x0, ext.x1, 30), np.linspace(ext.y0, ext.y1, 12))
inside = tp.contains_points(np.c_[gx.ravel(), gy.ravel()])
print(int(inside.sum()))
for row in inside.reshape(gy.shape)[::-1][::2]:
    print("".join("#" if v else "." for v in row))
fig, ax = plt.subplots(figsize=(4, 2))
img = ax.imshow(np.linspace(0, 1, 100).reshape(1, -1), extent=(ext.x0, ext.x1, ext.y0, ext.y1), aspect="auto")
patch = PathPatch(tp, transform=ax.transData, fc="none")
ax.add_patch(patch)
img.set_clip_path(patch)
fig.savefig("textclip.png")
print(type(img.get_clip_path()).__name__)
