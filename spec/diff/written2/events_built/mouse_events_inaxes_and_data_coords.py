# Build MouseEvents at pixel positions over two axes (one log, one twinned) and read inaxes, xdata, ydata and modifiers.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent, MouseButton

fig, (a1, a2) = plt.subplots(1, 2, figsize=(6, 3), dpi=100)
a1.set_xlim(0, 10)
a1.set_ylim(-1, 1)
a2.set_xscale("log")
a2.set_xlim(1, 1000)
a2.set_ylim(0, 5)
tw = a2.twinx()
tw.set_ylim(100, 200)
fig.canvas.draw()
points = [(50, 50), (150, 150), (400, 150), (590, 290), (0, 0), (299, 150)]
for x, y in points:
    ev = MouseEvent("motion_notify_event", fig.canvas, x, y)
    name = None if ev.inaxes is None else ("a1" if ev.inaxes is a1 else "a2" if ev.inaxes is a2 else "twin")
    xd = None if ev.xdata is None else round(ev.xdata, 3)
    yd = None if ev.ydata is None else round(ev.ydata, 3)
    print((x, y), name, xd, yd)
ev = MouseEvent("button_press_event", fig.canvas, 100, 100, button=MouseButton.RIGHT, key="shift", dblclick=True,
                modifiers={"shift", "ctrl"})
print(ev.button, ev.button == 3, ev.key, ev.dblclick, sorted(ev.modifiers), ev.step)
sc = MouseEvent("scroll_event", fig.canvas, 100, 100, button="up", step=2.5)
print(sc.button, sc.step, sc.name, sc.canvas is fig.canvas)
print(str(ev)[:60])
print([b.name for b in MouseButton], int(MouseButton.BACK))
hits = fig.canvas.inaxes((400, 150))
print(hits is tw, hits is a2)
a1.set_navigate(False)
print(MouseEvent("motion_notify_event", fig.canvas, 50, 50).inaxes is a1)
fig.savefig("events.png")
