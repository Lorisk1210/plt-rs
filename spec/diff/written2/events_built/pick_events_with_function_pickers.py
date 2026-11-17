# Function pickers on lines and a collection: the picker callable decides hits and attaches its own properties to the PickEvent.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent

fig, ax = plt.subplots(figsize=(5, 4), dpi=100)
x = np.linspace(0, 10, 101)
ln, = ax.plot(x, np.sin(x))
sc = ax.scatter(x[::10], np.cos(x[::10]), s=30)


def nearest_x(artist, mouseevent):
    if mouseevent.xdata is None:
        return False, {}
    xs = artist.get_xdata()
    i = int(np.argmin(np.abs(xs - mouseevent.xdata)))
    hit = abs(artist.get_ydata()[i] - mouseevent.ydata) < 0.2
    return hit, dict(index=i, distance=round(float(abs(artist.get_ydata()[i] - mouseevent.ydata)), 4))


def in_band(artist, mouseevent):
    offs = artist.get_offsets()
    if mouseevent.ydata is None:
        return False, {}
    ind = np.nonzero(np.abs(offs[:, 1] - mouseevent.ydata) < 0.3)[0]
    return len(ind) > 0, dict(ind=ind)


ln.set_picker(nearest_x)
sc.set_picker(in_band)
fig.canvas.draw()
picked = []
fig.canvas.mpl_connect("pick_event", lambda e: picked.append(
    (type(e.artist).__name__, getattr(e, "index", None), getattr(e, "distance", None),
     getattr(e, "ind", np.array([])).tolist(), e.mouseevent.button)))
for xd, yd in [(1.0, 0.85), (5.0, -0.9), (3.0, 0.0), (9.0, 0.4)]:
    px, py = ax.transData.transform((xd, yd))
    MouseEvent("button_press_event", fig.canvas, px, py, button=1)._process()
for p in picked:
    print(p)
print(ln.get_picker() is nearest_x, sc.pickable(), ln.pickable())
ln.set_picker(False)
picked.clear()
px, py = ax.transData.transform((1.0, 0.85))
MouseEvent("button_press_event", fig.canvas, px, py, button=1)._process()
print(picked, ln.pickable())
fig.savefig("pickers.png")
