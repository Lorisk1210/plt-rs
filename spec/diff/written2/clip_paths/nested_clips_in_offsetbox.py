# Clipping inside offset boxes: DrawingArea with clip=True, an AnnotationBbox, and children that overflow it.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.offsetbox import DrawingArea, AnnotationBbox, AnchoredOffsetbox
from matplotlib.patches import Circle, Rectangle

fig, ax = plt.subplots(figsize=(4, 4))
da = DrawingArea(60, 40, clip=True)
da.add_artist(Circle((30, 20), 35, fc="C1"))
da.add_artist(Rectangle((-10, -10), 30, 30, fc="C2"))
ab = AnnotationBbox(da, (0.5, 0.5), xybox=(0.7, 0.8), xycoords="data", boxcoords="axes fraction",
                    arrowprops=dict(arrowstyle="->"))
ax.add_artist(ab)
da2 = DrawingArea(60, 40, clip=False)
da2.add_artist(Circle((30, 20), 35, fc="C3", alpha=0.4))
anch = AnchoredOffsetbox(loc="lower left", child=da2, frameon=True)
ax.add_artist(anch)
fig.canvas.draw()
r = fig.canvas.get_renderer()
print(da.clip_children, da2.clip_children, [round(v, 2) for v in da.get_window_extent(r).bounds])
print([c.get_clip_on() for c in da.get_children()])
print([type(c.get_clip_path()).__name__ for c in da.get_children()])
print([round(v, 2) for v in anch.get_window_extent(r).bounds])
print([round(v, 2) for v in ab.get_window_extent(r).bounds])
print([round(v, 2) for v in ab.get_tightbbox(r).bounds])
fig.savefig("offset_clip.png")
da.clip_children = False
fig.canvas.draw()
print([type(c.get_clip_path()).__name__ for c in da.get_children()])
fig.savefig("offset_clip2.png")
