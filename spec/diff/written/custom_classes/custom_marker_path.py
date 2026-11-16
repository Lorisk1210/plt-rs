# Custom markers from a Path (arrow shape) and from vertices, rotated with MarkerStyle transform.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.path import Path
from matplotlib.markers import MarkerStyle
from matplotlib.transforms import Affine2D

arrow = Path([(0, -1), (0.3, -0.2), (0.15, -0.2), (0.15, 1), (-0.15, 1), (-0.15, -0.2), (-0.3, -0.2), (0, -1)])
fig, ax = plt.subplots(figsize=(6, 4))
for k, angle in enumerate(range(0, 360, 45)):
    m = MarkerStyle(arrow, transform=Affine2D().rotate_deg(angle))
    ax.plot(k, 0, marker=m, ms=25, color="C0")
ax.plot(range(8), [1] * 8, marker=[(0, 0), (1, 0), (0.5, 1)], ms=15, ls="", color="C2")
ax.set_ylim(-1, 2)
print(len(ax.lines))
fig.savefig("arrows.png")
