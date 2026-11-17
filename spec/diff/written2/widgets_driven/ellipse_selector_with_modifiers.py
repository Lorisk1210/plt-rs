# EllipseSelector with state modifiers: square and center keys held during drags, rotation, and the keypress state set.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent, KeyEvent
from matplotlib.widgets import EllipseSelector

fig, ax = plt.subplots(figsize=(4, 4), dpi=100)
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
fig.canvas.draw()
sel = EllipseSelector(ax, lambda a, b: None, interactive=True, useblit=False)


def drag(p0, p1, key=None):
    if key:
        KeyEvent("key_press_event", fig.canvas, key, *ax.transData.transform(p0))._process()
    x0, y0 = ax.transData.transform(p0)
    x1, y1 = ax.transData.transform(p1)
    MouseEvent("button_press_event", fig.canvas, x0, y0, 1, key=key)._process()
    MouseEvent("motion_notify_event", fig.canvas, x1, y1, 1, key=key)._process()
    MouseEvent("button_release_event", fig.canvas, x1, y1, 1, key=key)._process()
    if key:
        KeyEvent("key_release_event", fig.canvas, key, *ax.transData.transform(p1))._process()


def show(tag):
    print(tag, [round(v, 2) for v in sel.extents], sorted(sel._state))


drag((2, 2), (6, 4))
show("plain")
sel.clear()
drag((2, 2), (6, 4), key="shift")
show("square")
sel.clear()
drag((5, 5), (7, 6), key="control")
show("center")
sel.clear()
drag((5, 5), (7, 6), key="ctrl+shift")
show("both")
sel.add_state("rotate")
print(sorted(sel._state))
sel.rotation = 30
print(round(sel.rotation, 2), [round(v, 2) for v in sel.extents])
sel.remove_state("rotate")
try:
    sel.add_state("fly")
except ValueError as e:
    print("ValueError", str(e)[:50])
print(type(sel._selection_artist).__name__, sel.get_visible())
fig.savefig("ellipse.png")
