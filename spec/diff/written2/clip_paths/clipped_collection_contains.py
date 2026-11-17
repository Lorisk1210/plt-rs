# Does picking honour clipping? contains() on clipped lines, scatters and patches at points inside and outside the clip.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent
from matplotlib.patches import Rectangle

fig, ax = plt.subplots(figsize=(4, 4))
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
clip = Rectangle((0, 0), 5, 10, transform=ax.transData, fc="none")
ax.add_patch(clip)
ln, = ax.plot([0, 10], [5, 5], lw=4, picker=5)
sc = ax.scatter([2, 8], [2, 8], s=200, picker=True)
pt = ax.add_patch(Rectangle((6, 6), 3, 3, fc="C2"))
for a in (ln, sc, pt):
    a.set_clip_path(clip)
fig.canvas.draw()


def event_at(x, y):
    px, py = ax.transData.transform((x, y))
    return MouseEvent("button_press_event", fig.canvas, px, py, button=1)


for x, y in [(2, 5), (8, 5), (2, 2), (8, 8), (7, 7)]:
    ev = event_at(x, y)
    print((x, y), ln.contains(ev)[0], sc.contains(ev)[0], sc.contains(ev)[1].get("ind", np.array([])).tolist(),
          pt.contains(ev)[0])
picked = []
fig.canvas.mpl_connect("pick_event", lambda e: picked.append(type(e.artist).__name__))
for x, y in [(2, 5), (8, 5), (8, 8)]:
    fig.canvas.callbacks.process("button_press_event", event_at(x, y))
print(picked)
print(len(picked), sorted(set(picked)))
fig.savefig("clipped_pick.png")
