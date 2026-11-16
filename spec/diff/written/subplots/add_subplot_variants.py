# Mix add_subplot call styles (111, (2,2,3), keyword projection) in one figure as old scripts do.
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(8, 6))
ax_a = fig.add_subplot(221)
ax_b = fig.add_subplot(2, 2, 2, facecolor="#f0f0f0")
ax_c = fig.add_subplot(2, 2, 3, title="from keywords")
ax_d = fig.add_subplot(2, 2, 4, projection="polar")
ax_a.plot([1, 2, 3], [3, 1, 2])
ax_b.bar(["x", "y"], [2, 5])
ax_c.scatter([1, 2, 3], [1, 4, 9])
theta = np.linspace(0, 2 * np.pi, 100)
ax_d.plot(theta, 1 + 0.5 * np.cos(3 * theta))
print(type(ax_d).__name__, ax_c.get_title())
print(ax_a.get_subplotspec().get_geometry())
print(len(fig.axes))
fig.savefig("variants.png")
