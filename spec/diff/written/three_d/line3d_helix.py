# Helix and its projection onto the floor with plot() in 3-D and zdir='z' offsets.
import numpy as np
import matplotlib.pyplot as plt

t = np.linspace(0, 6 * np.pi, 400)
x, y, z = np.cos(t), np.sin(t), t / (2 * np.pi)
fig = plt.figure(figsize=(5, 5))
ax = fig.add_subplot(projection="3d")
ax.plot(x, y, z, lw=2, label="helix")
ax.plot(x, y, zs=0, zdir="z", color="gray", ls="--", label="shadow")
ax.plot(x, z, zs=1.5, zdir="y", color="C1", alpha=0.5)
ax.set_ylim(-1.5, 1.5)
ax.legend()
print([round(v, 2) for v in ax.get_zlim()])
fig.savefig("helix.png")
