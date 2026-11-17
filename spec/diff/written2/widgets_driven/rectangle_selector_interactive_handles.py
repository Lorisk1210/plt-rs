# RectangleSelector driven by synthetic drags: draw, move by the centre handle, resize by a corner, read extents and corners.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent
from matplotlib.widgets import RectangleSelector

fig, ax = plt.subplots(figsize=(4, 4), dpi=100)
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
fig.canvas.draw()
done = []
sel = RectangleSelector(ax, lambda e1, e2: done.append((round(e1.xdata, 2), round(e2.xdata, 2))),
                        interactive=True, useblit=False, button=[1], minspanx=0.1, minspany=0.1)


def drag(p0, p1, button=1, steps=3):
    x0, y0 = ax.transData.transform(p0)
    x1, y1 = ax.transData.transform(p1)
    MouseEvent("button_press_event", fig.canvas, x0, y0, button)._process()
    for k in range(1, steps + 1):
        MouseEvent("motion_notify_event", fig.canvas, x0 + (x1 - x0) * k / steps, y0 + (y1 - y0) * k / steps, button)._process()
    MouseEvent("button_release_event", fig.canvas, x1, y1, button)._process()


def show(tag):
    print(tag, [round(v, 2) for v in sel.extents], [[round(v, 2) for v in c] for c in sel.corners],
          [round(v, 2) for v in sel.center])


drag((2, 2), (6, 5))
show("drawn")
drag((4, 3.5), (5, 4.5))
show("moved")
drag((7, 6), (8, 8))
show("resized")
drag((1, 1), (1.05, 1.02))
show("tiny")
print(done)
sel.extents = (1, 3, 1, 3)
show("set")
print(sel.active, sel.visible if hasattr(sel, "visible") else sel.get_visible())
sel.set_active(False)
drag((5, 5), (9, 9))
show("inactive")
print(len(sel.artists), [type(a).__name__ for a in sel.artists])
fig.savefig("rect_selector.png")
