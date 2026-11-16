# A line whose colour changes with a third variable using LineCollection, with a colour bar.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.collections import LineCollection

t = np.linspace(0, 10, 400)
x = np.cos(t) * t
y = np.sin(t) * t
speed = np.hypot(np.gradient(x), np.gradient(y))
pts = np.c_[x, y].reshape(-1, 1, 2)
segs = np.concatenate([pts[:-1], pts[1:]], axis=1)
fig, ax = plt.subplots(figsize=(5, 5))
lc = LineCollection(segs, cmap="turbo", array=speed[:-1], linewidth=2)
ax.add_collection(lc)
ax.autoscale()
ax.set_aspect("equal")
fig.colorbar(lc, ax=ax, label="speed")
print(round(float(speed.max()), 5))
fig.savefig("spiral_speed.png")
