# transform_angles for text following a data direction, under non-equal aspect; compare with arctan of display deltas.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(6, 3), dpi=100)
ax.set_xlim(0, 10)
ax.set_ylim(0, 1)
fig.canvas.draw()
angles = np.array([0.0, 10.0, 45.0, 89.0])
pts = np.array([[5.0, 0.5]] * len(angles))
display = ax.transData.transform_angles(angles, pts)
print(np.round(display, 6).tolist())
d = ax.transData.transform([[5.0, 0.5], [6.0, 0.5 + np.tan(np.radians(10))]])
print(round(float(np.degrees(np.arctan2(*(d[1] - d[0])[::-1]))), 6))
rad = ax.transData.transform_angles(np.radians(angles), pts, radians=True)
print(np.round(np.degrees(rad), 6).tolist())
for a in angles:
    ax.text(5, 0.5, f"{a:g}°", rotation=a, transform_rotates_text=True, rotation_mode="anchor")
fig.canvas.draw()
print([round(t.get_rotation(), 6) for t in ax.texts])
print([t.get_transform_rotates_text() for t in ax.texts])
fig.savefig("angles.png")
ax.set_ylim(0, 2)
fig.canvas.draw()
print([round(t.get_rotation(), 6) for t in ax.texts])
print(np.round(ax.transData.transform_angles(angles, pts), 6).tolist())
ax.invert_xaxis()
fig.canvas.draw()
print([round(t.get_rotation(), 6) for t in ax.texts])
