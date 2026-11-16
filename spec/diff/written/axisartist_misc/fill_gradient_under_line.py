# Gradient fill under a curve: an imshow gradient clipped by the polygon under the line.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Polygon

x = np.linspace(0, 10, 200)
y = 2 + np.sin(x) + 0.2 * x
fig, ax = plt.subplots(figsize=(7, 3.5))
line, = ax.plot(x, y, color="navy", lw=2)
grad = np.linspace(0, 1, 256).reshape(-1, 1)
im = ax.imshow(grad, extent=[x.min(), x.max(), 0, y.max()], aspect="auto", cmap="Blues", origin="lower", alpha=0.8)
poly = Polygon(np.r_[np.c_[x, y], [[x.max(), 0], [x.min(), 0]]], closed=True, fc="none", ec="none")
ax.add_patch(poly)
im.set_clip_path(poly)
ax.set_ylim(0, y.max() * 1.1)
print(round(float(y.max()), 4))
fig.savefig("gradient_fill.png")
