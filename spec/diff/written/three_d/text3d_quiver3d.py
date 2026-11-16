# Coordinate frame with quiver arrows in 3-D and text labels at the tips.
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(5, 5))
ax = fig.add_subplot(projection="3d")
origin = [0, 0, 0]
for (u, v, w), name, color in [((1, 0, 0), "x", "r"), ((0, 1, 0), "y", "g"), ((0, 0, 1), "z", "b")]:
    ax.quiver(*origin, u, v, w, color=color, arrow_length_ratio=0.15)
    ax.text(u * 1.1, v * 1.1, w * 1.1, name, color=color, fontsize=14)
ax.text2D(0.05, 0.95, "frame", transform=ax.transAxes)
ax.set_xlim(-0.2, 1.2)
ax.set_ylim(-0.2, 1.2)
ax.set_zlim(-0.2, 1.2)
print(len(ax.texts))
fig.savefig("frame.png")
