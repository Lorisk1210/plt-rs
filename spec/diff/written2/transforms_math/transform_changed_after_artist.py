# Change an Affine2D in place after artists use it (line, patch, text, collection) and verify they move at the next draw.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.transforms import Affine2D
from matplotlib.patches import Circle
from matplotlib.collections import PathCollection
from matplotlib.path import Path

fig, ax = plt.subplots(figsize=(4, 4), dpi=60)
ax.set_xlim(-5, 5)
ax.set_ylim(-5, 5)
shift = Affine2D()
line, = ax.plot([0, 1], [0, 1], transform=shift + ax.transData)
circ = ax.add_patch(Circle((0, 0), 0.5, transform=shift + ax.transData))
txt = ax.text(0, 0, "T", transform=shift + ax.transData)
coll = ax.scatter([0, 1], [1, 0], transform=shift + ax.transData)


def where():
    fig.canvas.draw()
    return ([round(v, 3) for v in line.get_window_extent().bounds[:2]],
            [round(v, 3) for v in circ.get_window_extent().bounds[:2]],
            [round(v, 3) for v in txt.get_window_extent().bounds[:2]],
            [round(v, 3) for v in coll.get_window_extent().bounds[:2]])


print(where())
shift.translate(2, -1)
print(where())
shift.clear().rotate_deg(90)
print(where())
fig.savefig("moved.png")
print(type(line.get_transform()).__name__, line.get_transform().contains_branch(shift))
