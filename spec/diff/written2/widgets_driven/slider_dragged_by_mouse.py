# Drag a Slider's handle and a RangeSlider's handles with mouse events, including outside the axes and with orientation vertical.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent
from matplotlib.widgets import Slider, RangeSlider

fig = plt.figure(figsize=(5, 4), dpi=100)
hs = Slider(fig.add_axes([0.15, 0.8, 0.7, 0.08]), "h", 0, 100, valinit=50)
vs = Slider(fig.add_axes([0.1, 0.1, 0.06, 0.6]), "v", -1, 1, valinit=0, orientation="vertical")
rs = RangeSlider(fig.add_axes([0.3, 0.4, 0.6, 0.08]), "r", 0, 10, valinit=(2, 8))
fig.canvas.draw()
log = []
hs.on_changed(lambda v: log.append(("h", round(v, 3))))
vs.on_changed(lambda v: log.append(("v", round(v, 3))))
rs.on_changed(lambda v: log.append(("r", tuple(round(x, 3) for x in v))))


def drag(ax, fx0, fy0, fx1, fy1):
    bb = ax.get_window_extent()
    p0 = (bb.x0 + fx0 * bb.width, bb.y0 + fy0 * bb.height)
    p1 = (bb.x0 + fx1 * bb.width, bb.y0 + fy1 * bb.height)
    MouseEvent("button_press_event", fig.canvas, *p0, 1)._process()
    MouseEvent("motion_notify_event", fig.canvas, *p1, 1)._process()
    MouseEvent("button_release_event", fig.canvas, *p1, 1)._process()


drag(hs.ax, 0.5, 0.5, 0.75, 0.5)
drag(hs.ax, 0.75, 0.5, 1.5, 0.5)
drag(vs.ax, 0.5, 0.5, 0.5, 0.1)
drag(rs.ax, 0.2, 0.5, 0.05, 0.5)
drag(rs.ax, 0.8, 0.5, 0.6, 0.5)
drag(rs.ax, 0.3, 0.5, 0.9, 0.5)
for entry in log:
    print(entry)
print(round(hs.val, 3), round(vs.val, 3), tuple(round(x, 3) for x in rs.val))
print(hs.valtext.get_text(), vs.valtext.get_text(), rs.valtext.get_text())
print(hs.drag_active, rs.drag_active)
fig.savefig("dragged.png")
