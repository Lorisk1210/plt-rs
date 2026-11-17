# axes_grid1.anchored_artists: AnchoredDrawingArea, AnchoredAuxTransformBox, AnchoredEllipse and AnchoredSizeBar options.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Circle, Ellipse
from mpl_toolkits.axes_grid1.anchored_artists import (AnchoredDrawingArea, AnchoredAuxTransformBox, AnchoredSizeBar)
import matplotlib.font_manager as fm

fig, ax = plt.subplots(figsize=(5, 4), dpi=80)
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
ada = AnchoredDrawingArea(40, 20, 0, 0, loc="upper right", pad=0.1, frameon=False)
ada.da.add_artist(Circle((10, 10), 10, fc="C0"))
ada.da.add_artist(Circle((30, 10), 5, fc="C1"))
ax.add_artist(ada)
box = AnchoredAuxTransformBox(ax.transData, loc="upper left")
box.drawing_area.add_artist(Ellipse((0, 0), width=2, height=1, angle=30))
ax.add_artist(box)
bar = AnchoredSizeBar(ax.transData, 2.5, "2.5 units", "lower right", pad=0.3, color="k", frameon=True,
                      size_vertical=0.2, label_top=True, fontproperties=fm.FontProperties(size=9), sep=4)
ax.add_artist(bar)
fig.canvas.draw()
r = fig.canvas.get_renderer()
for a in (ada, box, bar):
    print(type(a).__name__, [round(v, 2) for v in a.get_window_extent(r).bounds])
print(len(ada.da.get_children()), type(box.drawing_area).__name__, bar.size_bar is not None, bar.txt_label.get_text())
ax.set_xlim(0, 20)
fig.canvas.draw()
print([round(v, 2) for v in box.get_window_extent(r).bounds], [round(v, 2) for v in bar.get_window_extent(r).bounds])
print([type(c).__name__ for c in bar.get_children()])
fig.savefig("anchored_grid1.png")
