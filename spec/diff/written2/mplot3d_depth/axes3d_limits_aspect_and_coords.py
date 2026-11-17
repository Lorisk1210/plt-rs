# Axes3D limits, set_box_aspect, set_aspect variants, margins, autoscale per axis, invert and format_coord readouts.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(5, 5), dpi=60)
ax = fig.add_subplot(projection="3d")
t = np.linspace(0, 4 * np.pi, 100)
ax.plot(np.cos(t), np.sin(t), t / 4)
fig.canvas.draw()
print([np.round(f(), 4).tolist() for f in (ax.get_xlim, ax.get_ylim, ax.get_zlim)])
print(np.round(ax.get_box_aspect(), 4).tolist() if ax.get_box_aspect() is not None else None)
for aspect in ("auto", "equal", "equalxy", "equalxz", "equalyz"):
    ax.set_aspect(aspect)
    fig.canvas.draw()
    print(aspect, [np.round(f(), 3).tolist() for f in (ax.get_xlim, ax.get_ylim, ax.get_zlim)],
          np.round(ax._box_aspect, 3).tolist())
ax.set_aspect("auto")
ax.set_box_aspect((2, 1, 0.5), zoom=0.8)
print(np.round(ax._box_aspect, 4).tolist())
ax.margins(0.2, 0, 0.1)
ax.autoscale_view()
print(ax.margins(), [np.round(f(), 3).tolist() for f in (ax.get_xlim, ax.get_ylim, ax.get_zlim)])
ax.invert_zaxis()
print(ax.zaxis_inverted(), np.round(ax.get_zlim(), 3).tolist(), ax.get_zbound())
ax.set_zlim(0, 5)
ax.set_xlim3d(-2, 2)
print(np.round(ax.get_w_lims(), 3).tolist())
fig.canvas.draw()
print(ax.format_coord(0.0, 0.0))
print(ax.get_zscale(), ax.zaxis.get_major_locator().__class__.__name__)
fig.savefig("limits3d.png")
