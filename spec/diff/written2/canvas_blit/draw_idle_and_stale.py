# Watch stale propagation without pyplot: set properties, read fig.stale and ax.stale, draw, and use a stale_callback.
from matplotlib.figure import Figure
from matplotlib.backends.backend_agg import FigureCanvasAgg

fig = Figure(figsize=(3, 2))
canvas = FigureCanvasAgg(fig)
ax = fig.add_subplot()
line, = ax.plot([0, 1], [0, 1])
log = []
fig.stale_callback = lambda artist, val: log.append(("fig", val))
print(fig.stale, ax.stale, line.stale)
canvas.draw()
print(fig.stale, ax.stale, line.stale)
line.set_color("red")
print(fig.stale, ax.stale, line.stale)
canvas.draw()
ax.set_title("t")
print(fig.stale, ax.stale)
canvas.draw_idle()
print(fig.stale)
canvas.draw()
line.set_visible(False)
print(line.stale, ax.stale, fig.stale)
print(log[:6], len(log))
fig.savefig("stale.png")
fig.stale_callback = None
ax.stale_callback is not None and print("axes has a callback")
text = fig.text(0.5, 0.5, "x")
print(text.stale, fig.stale)
canvas.draw()
print(text.stale, fig.stale, ax.stale)
