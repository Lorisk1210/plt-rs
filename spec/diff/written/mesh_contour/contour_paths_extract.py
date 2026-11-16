# Extract the vertices of a contour level (e.g. to measure the area inside an isoline).
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-2, 2, 201)
X, Y = np.meshgrid(x, x)
Z = X ** 2 + 2 * Y ** 2
fig, ax = plt.subplots(figsize=(4.5, 4.5))
cs = ax.contour(X, Y, Z, levels=[1.0])
path = cs.get_paths()[0]
v = path.vertices
area = 0.5 * abs(np.dot(v[:, 0], np.roll(v[:, 1], 1)) - np.dot(v[:, 1], np.roll(v[:, 0], 1)))
print(f"area {area:.3f} vs exact {np.pi / np.sqrt(2):.3f}")
ax.fill(v[:, 0], v[:, 1], alpha=0.3)
ax.set_aspect("equal")
fig.savefig("ellipse.png")
