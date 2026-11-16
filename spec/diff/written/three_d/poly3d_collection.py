# Build a pyramid from polygons with Poly3DCollection and set limits/aspect by hand.
import numpy as np
import matplotlib.pyplot as plt
from mpl_toolkits.mplot3d.art3d import Poly3DCollection

base = [(0, 0, 0), (1, 0, 0), (1, 1, 0), (0, 1, 0)]
apex = (0.5, 0.5, 1.0)
faces = [base] + [[base[i], base[(i + 1) % 4], apex] for i in range(4)]
fig = plt.figure(figsize=(5, 5))
ax = fig.add_subplot(projection="3d")
coll = Poly3DCollection(faces, facecolors=["gray", "C0", "C1", "C2", "C3"], edgecolors="k", alpha=0.8)
ax.add_collection3d(coll)
ax.set_xlim(0, 1)
ax.set_ylim(0, 1)
ax.set_zlim(0, 1)
ax.set_aspect("equal")
print(len(faces), ax.get_aspect())
fig.savefig("pyramid.png")
