# Interactive fit tool driven by synthetic clicks: clicks add points, a key press fits a line and annotates the slope.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent, KeyEvent

fig, ax = plt.subplots(figsize=(6, 4))
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
pts, = ax.plot([], [], "o")
fit_line, = ax.plot([], [], "r-")
xs, ys = [], []
def on_click(e):
    if e.inaxes is ax and e.button == 1:
        xs.append(e.xdata); ys.append(e.ydata)
        pts.set_data(xs, ys)
def on_key(e):
    if e.key == "f" and len(xs) > 1:
        k, b = np.polyfit(xs, ys, 1)
        fit_line.set_data([0, 10], [b, 10 * k + b])
        ax.set_title(f"slope {k:.3f}")
fig.canvas.mpl_connect("button_press_event", on_click)
fig.canvas.mpl_connect("key_press_event", on_key)
fig.canvas.draw()
for xd, yd in [(1, 2), (3, 3.1), (5, 4.2), (8, 6.0)]:
    px, py = ax.transData.transform((xd, yd))
    MouseEvent("button_press_event", fig.canvas, px, py, button=1)._process()
KeyEvent("key_press_event", fig.canvas, "f")._process()
print([round(v, 3) for v in xs], ax.get_title(), np.round(fit_line.get_ydata(), 3).tolist())
fig.savefig("fit_tool.png", dpi=50)
