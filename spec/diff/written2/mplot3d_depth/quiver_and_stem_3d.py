# 3-D quiver with normalize, length, pivot and arrow_length_ratio; stem plots along each orientation; segment counts and extents.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

x, y, z = np.meshgrid(np.arange(0, 1.1, 0.5), np.arange(0, 1.1, 0.5), np.arange(0, 1.1, 0.5))
u, v, w = np.sin(np.pi * x), -np.cos(np.pi * y), z * 0.5
fig = plt.figure(figsize=(9, 6), dpi=50)
for k, kw in enumerate([dict(), dict(normalize=True, length=0.2), dict(pivot="middle", length=0.3),
                        dict(pivot="tip", arrow_length_ratio=0.5, length=0.1)]):
    ax = fig.add_subplot(2, 3, k + 1, projection="3d")
    q = ax.quiver(x, y, z, u, v, w, **kw)
    segs = q._segments3d
    print(sorted(kw), len(segs), np.round(np.asarray(segs[1]), 3).tolist())
theta = np.linspace(0, 2 * np.pi, 8)
for k, orient in enumerate(("z", "x")):
    ax = fig.add_subplot(2, 3, 5 + k, projection="3d")
    markers, stems, base = ax.stem(np.cos(theta), np.sin(theta), theta, orientation=orient, bottom=0.5)
    fig.canvas.draw()
    print(orient, type(stems).__name__, len(stems.get_segments()), np.round(np.array(markers.get_data_3d())[:, 0], 3).tolist())
    print(np.round(np.array(base.get_data_3d())[:, :2], 3).tolist())
fig.canvas.draw()
fig.savefig("quiver3d.png")
ax = fig.axes[0]
q2 = ax.quiver([0], [0], [0], [1], [1], [1], colors=["r"], lw=2)
fig.canvas.draw()
print(len(q2._segments3d), np.round(q2.get_color()[0], 3).tolist() if len(q2.get_color()) else None)
print([round(v, 3) for v in ax.get_xlim()], [round(v, 3) for v in ax.get_zlim()])
fig.savefig("quiver3d_b.png")
