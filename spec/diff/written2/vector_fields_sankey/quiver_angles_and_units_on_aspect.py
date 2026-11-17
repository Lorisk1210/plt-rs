# Quiver angles="uv" vs "xy" on an axes with unequal aspect and log scale, angle arrays in degrees, and the arrow directions read back.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, axs = plt.subplots(1, 3, figsize=(9, 3), dpi=50)
X, Y = np.meshgrid([1, 2, 3], [1, 2])
U = np.ones_like(X, float)
V = np.ones_like(X, float)
for ax, angles in zip(axs, ("uv", "xy", np.array([[0, 45, 90], [135, 180, 270]]))):
    ax.set_xlim(0, 4)
    ax.set_ylim(0, 20)
    q = ax.quiver(X, Y * 5, U, V, angles=angles, scale_units="width", scale=8)
    fig.canvas.draw()
    verts = [p.vertices for p in q.get_paths()]
    tip = verts[0][np.argmax(np.hypot(*verts[0].T))]
    print(str(angles)[:10], len(verts), round(float(np.degrees(np.arctan2(tip[1], tip[0]))), 2),
          np.round(np.ptp(verts[0], axis=0), 3).tolist())
fig2, ax2 = plt.subplots(figsize=(4, 3), dpi=50)
ax2.set_xscale("log")
ax2.set_xlim(1, 1000)
ax2.set_ylim(0, 1)
q = ax2.quiver([10, 100], [0.5, 0.5], [1, 1], [1, 1], angles="xy", scale_units="xy", scale=0.1)
fig2.canvas.draw()
print([np.round(np.ptp(p.vertices, axis=0), 3).tolist() for p in q.get_paths()])
fig.savefig("quiver_angles.png")
ax2.set_xlim(1, 100)
fig2.canvas.draw()
print([np.round(np.ptp(p.vertices, axis=0), 3).tolist() for p in q.get_paths()])
q.set_UVC([0, 1], [1, 0])
fig2.canvas.draw()
print([np.round(np.ptp(p.vertices, axis=0), 3).tolist() for p in q.get_paths()])
