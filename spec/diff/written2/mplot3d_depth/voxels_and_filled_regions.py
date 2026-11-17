# ax.voxels with a boolean array, coloured faces and explicit corner coordinates; the returned dict of faces per voxel.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

x, y, z = np.indices((4, 4, 4))
filled = (x + y + z) % 3 == 0
colors = np.empty(filled.shape, dtype=object)
colors[filled] = "tab:blue"
colors[x == 0] = "tab:red"
fig = plt.figure(figsize=(8, 4), dpi=60)
ax1 = fig.add_subplot(1, 2, 1, projection="3d")
faces = ax1.voxels(filled, facecolors=colors, edgecolor="k")
print(len(faces), sorted(faces)[:3], type(next(iter(faces.values()))).__name__)
ax2 = fig.add_subplot(1, 2, 2, projection="3d")
r, theta, zz = np.mgrid[0:1:4j, 0:np.pi:5j, 0:1:3j]
X, Y = r * np.cos(theta), r * np.sin(theta)
solid = np.ones((3, 4, 2), bool)
solid[1, 1, :] = False
faces2 = ax2.voxels(X, Y, zz, solid, facecolors="orange", shade=False)
fig.canvas.draw()
print(len(faces2), sorted(faces2)[-1])
print([round(v, 3) for v in ax1.get_xlim3d()], [round(v, 3) for v in ax2.get_xlim3d()], [round(v, 3) for v in ax2.get_ylim3d()])
key = sorted(faces)[0]
print(np.round(faces[key].get_facecolor()[0], 3).tolist(), len(faces[key].get_paths()))
try:
    ax1.voxels(np.ones((2, 2)))
except ValueError as e:
    print("ValueError", str(e)[:60])
fig.savefig("voxels.png")
