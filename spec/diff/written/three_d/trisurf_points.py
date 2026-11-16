# Surface from scattered points with plot_trisurf, and a view from above saved too.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(6)
x = rng.uniform(-1, 1, 300)
y = rng.uniform(-1, 1, 300)
z = x ** 2 - y ** 2
fig = plt.figure(figsize=(6, 5))
ax = fig.add_subplot(projection="3d")
ax.plot_trisurf(x, y, z, cmap="viridis", edgecolor="none")
fig.savefig("trisurf.png")
ax.view_init(elev=90, azim=-90)
fig.savefig("trisurf_top.png")
ax.view_init(elev=10, azim=45, roll=15)
fig.savefig("trisurf_roll.png")
print(ax.roll)
