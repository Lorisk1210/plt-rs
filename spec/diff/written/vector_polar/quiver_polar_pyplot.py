# pyplot-only quiver of a unit-vector field plus a polar subplot of the same directions.
import numpy as np
import matplotlib.pyplot as plt

angles = np.linspace(0, 2 * np.pi, 12, endpoint=False)
plt.figure(figsize=(8, 4))
plt.subplot(1, 2, 1)
plt.quiver(np.cos(angles), np.sin(angles), np.cos(angles + np.pi / 2), np.sin(angles + np.pi / 2), angles, cmap="twilight")
plt.axis("equal")
plt.title("tangents")
plt.subplot(1, 2, 2, projection="polar")
plt.plot(angles, np.ones_like(angles), "o")
plt.title("directions")
plt.tight_layout()
plt.savefig("tangents.png")
print(plt.gca().name)
