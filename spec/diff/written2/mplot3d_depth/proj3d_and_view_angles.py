# proj3d projection matrices for view_init(elev, azim, roll), focal lengths and orthographic projection; projected points compared.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d import proj3d

fig = plt.figure(figsize=(4, 4), dpi=60)
ax = fig.add_subplot(projection="3d")
ax.set(xlim=(0, 1), ylim=(0, 1), zlim=(0, 1))
pts = np.array([[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1], [1, 1, 1]], float)
for elev, azim, roll in [(30, -60, 0), (0, 0, 0), (90, 0, 0), (20, 45, 30), (-30, 120, -15)]:
    ax.view_init(elev=elev, azim=azim, roll=roll)
    M = ax.get_proj()
    xs, ys, zs = proj3d.proj_transform(pts[:, 0], pts[:, 1], pts[:, 2], M)
    print((elev, azim, roll), np.round(xs, 4).tolist(), np.round(ys, 4).tolist())
for proj, fl in [("persp", 1), ("persp", 0.2), ("ortho", None), ("persp", None)]:
    ax.set_proj_type(proj, focal_length=fl)
    ax.view_init(30, -60)
    M = ax.get_proj()
    xs, ys, _ = proj3d.proj_transform(pts[:, 0], pts[:, 1], pts[:, 2], M)
    print(proj, fl, round(ax._focal_length, 4), np.round(xs[-1], 4), np.round(ys[-1], 4))
print(np.round(proj3d.world_transformation(0, 1, 0, 2, 0, 4), 3).tolist())
print(np.round(proj3d._ortho_transformation(-1, 1), 3).tolist())
ax.view_init(30, -60)
fig.canvas.draw()
print(ax.format_coord(0.01, 0.02))
ax.view_init(elev=10, azim=200, vertical_axis="y")
print(ax.elev, ax.azim, np.round(ax.get_proj()[0], 4).tolist())
ax.view_init(30, -60, vertical_axis="z")
print(ax.elev, ax.azim, ax.roll, ax.get_proj().shape)
fig.savefig("proj.png")
