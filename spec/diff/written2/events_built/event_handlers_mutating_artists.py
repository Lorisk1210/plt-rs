# Handlers that add, move and remove artists while events are processed; the figure after each event, drawn and saved.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent, KeyEvent

fig, ax = plt.subplots(figsize=(4, 4), dpi=100)
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
marks = []


def on_click(event):
    if event.inaxes is not ax:
        return
    if event.button == 1:
        marks.append(ax.plot(event.xdata, event.ydata, "o", ms=8)[0])
    elif event.button == 3 and marks:
        marks.pop(0).remove()
    fig.canvas.draw_idle()


def on_key(event):
    if event.key == "c":
        for m in marks:
            m.set_color("red")
    elif event.key == "x":
        for m in marks:
            xd, yd = m.get_data()
            m.set_data(np.asarray(xd) + 1, yd)
    fig.canvas.draw_idle()


fig.canvas.mpl_connect("button_press_event", on_click)
fig.canvas.mpl_connect("key_press_event", on_key)
script = [("click", 1, (2, 2)), ("click", 1, (5, 5)), ("click", 1, (8, 3)), ("key", "c", None),
          ("click", 3, (0, 0)), ("key", "x", None), ("click", 1, (1, 9))]
for kind, what, pos in script:
    if kind == "click":
        px, py = ax.transData.transform(pos)
        MouseEvent("button_press_event", fig.canvas, px, py, button=what)._process()
    else:
        KeyEvent("key_press_event", fig.canvas, what, 10, 10)._process()
    fig.canvas.draw()
    print(kind, what, len(ax.lines), [(round(float(np.ravel(m.get_xdata())[0]), 2), m.get_color()) for m in marks])
fig.savefig("mutated.png")
