# Copy line properties to a new line with update_from, and copy.copy of a Path/transform.
import copy
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D

fig, ax = plt.subplots(figsize=(5, 3.5))
src, = ax.plot([0, 1, 2], [0, 1, 0], "s--", color="purple", lw=3, ms=8)
new = Line2D([0, 1, 2], [1, 2, 1])
new.update_from(src)
ax.add_line(new)
print(new.get_color(), new.get_linestyle(), new.get_marker(), new.get_linewidth())
path = copy.deepcopy(src.get_path())
print(path.vertices.tolist())
tr = copy.copy(ax.transData)
print(type(tr).__name__)
ax.set_ylim(-0.5, 2.5)
fig.savefig("copied.png")
