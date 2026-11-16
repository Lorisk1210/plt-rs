# Change patch geometry after creation (Rectangle bounds, Circle radius/center, Polygon xy, Wedge angles) and read back.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle, Circle, Polygon, Wedge

fig, ax = plt.subplots(figsize=(5, 5))
r = ax.add_patch(Rectangle((0, 0), 1, 1))
c = ax.add_patch(Circle((3, 3), 0.5, color="C1"))
p = ax.add_patch(Polygon([[0, 3], [1, 4], [0, 4]], color="C2"))
w = ax.add_patch(Wedge((3, 0.5), 1, 0, 90, color="C3"))
r.set_bounds(0.5, 0.5, 2, 1)
c.set_radius(0.8); c.set_center((3.5, 3.5))
p.set_xy([[0, 2.5], [1.5, 4], [0, 4.5]])
w.set_theta2(200); w.set_radius(1.2)
ax.set_xlim(-1, 5); ax.set_ylim(-1, 5)
fig.canvas.draw()
print(r.get_bbox().bounds, c.get_radius(), c.get_center(), p.get_xy().tolist(), w.theta2, w.r)
print([round(v, 3) for v in w.get_path().get_extents().bounds])
fig.savefig("geometry.png", dpi=50)
