# AnnotationBbox with an OffsetImage and a TextArea in several coordinate systems, made draggable, moved by synthetic mouse events.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent
from matplotlib.offsetbox import AnnotationBbox, OffsetImage, TextArea

fig, ax = plt.subplots(figsize=(5, 4))
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
img = OffsetImage(np.arange(64).reshape(8, 8), zoom=3, cmap="magma")
ab1 = AnnotationBbox(img, (2, 2), xybox=(60, 40), boxcoords="offset points", arrowprops=dict(arrowstyle="->"))
ab2 = AnnotationBbox(TextArea("axes"), (0.8, 0.8), xycoords="axes fraction", frameon=True)
ab3 = AnnotationBbox(TextArea("figure px"), (50, 50), xycoords="figure pixels", box_alignment=(0, 0))
for ab in (ab1, ab2, ab3):
    ax.add_artist(ab)
fig.canvas.draw()
r = fig.canvas.get_renderer()
for ab in (ab1, ab2, ab3):
    print([round(v, 1) for v in ab.get_window_extent(r).bounds], ab.xyann, ab.anncoords)
drag = ab2.draggable(True)
print(type(drag).__name__)
bb = ab2.get_window_extent(r)
x0, y0 = bb.x0 + bb.width / 2, bb.y0 + bb.height / 2
for name, x, y in [("button_press_event", x0, y0), ("motion_notify_event", x0 - 40, y0 - 30),
                   ("button_release_event", x0 - 40, y0 - 30)]:
    ev = MouseEvent(name, fig.canvas, x, y, button=1)
    fig.canvas.callbacks.process(name, ev)
fig.canvas.draw()
print(np.round(ab2.xyann, 3).tolist(), [round(v, 1) for v in ab2.get_window_extent(r).bounds])
print(img.get_zoom(), img.get_data().shape, ab1.offsetbox is img)
fig.savefig("annotation_bbox.png")
ab2.draggable(False)
print(ab2._draggable)
