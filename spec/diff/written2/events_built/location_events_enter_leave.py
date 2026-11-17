# axes_enter_event / axes_leave_event / figure_enter/leave driven by a sequence of motion events across the figure.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent, LocationEvent

fig, axs = plt.subplots(1, 2, figsize=(6, 3), dpi=100)
fig.canvas.draw()
log = []
for name in ("axes_enter_event", "axes_leave_event", "figure_enter_event", "figure_leave_event"):
    fig.canvas.mpl_connect(name, lambda e, n=name: log.append((n, None if e.inaxes is None else list(axs).index(e.inaxes))))
path = [(10, 10), (100, 150), (200, 150), (300, 150), (450, 150), (590, 150), (100, 150)]
for x, y in path:
    MouseEvent("motion_notify_event", fig.canvas, x, y)._process()
    log.append(("at", (x, y)))
LocationEvent("figure_leave_event", fig.canvas, 0, 0)._process()
for entry in log:
    print(entry)
print(LocationEvent("figure_enter_event", fig.canvas, 150, 150).inaxes is axs[0])
le = LocationEvent("x", fig.canvas, 150.5, 151.25)
print(le.x, le.y, round(le.xdata, 4), round(le.ydata, 4), le.name)
fig.savefig("enter_leave.png")
print(len(log))
print(sorted({n for n, _ in log}))
axs[1].remove()
log.clear()
for x, y in [(150, 150), (450, 150), (150, 150)]:
    MouseEvent("motion_notify_event", fig.canvas, x, y)._process()
print(log)
print(len(fig.axes), fig.canvas.inaxes((450, 150)))
