# Pick events on lines with picker tolerance: report which line was hit by synthetic clicks.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent

fig, ax = plt.subplots(figsize=(5, 4))
x = np.linspace(0, 1, 50)
lines = [ax.plot(x, x * k, label=f"slope {k}", picker=5)[0] for k in (1, 2, 3)]
picked = []
fig.canvas.mpl_connect("pick_event", lambda e: picked.append((e.artist.get_label(), list(e.ind[:2]))))
fig.canvas.draw()
for xd, yd in [(0.5, 1.0), (0.5, 0.5), (0.9, 0.1)]:
    px, py = ax.transData.transform((xd, yd))
    fig.pick(MouseEvent("button_press_event", fig.canvas, px, py, 1))
print(picked)
fig.savefig("pick.png")
