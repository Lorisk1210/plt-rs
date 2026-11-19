"""Three clusters

700,000 translucent points in three clusters, coloured by cluster
(``Axes.scatter``).
"""

import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(7)
n = 700_000
which = rng.integers(0, 3, n)
centers = np.array([[-1.3, 0.4], [1.1, 1.05], [0.25, -1.15]])
x = centers[which, 0] + rng.normal(scale=0.42, size=n)
y = centers[which, 1] + rng.normal(scale=0.34, size=n)

fig, ax = plt.subplots(figsize=(6.6, 4.3), dpi=100)
ax.scatter(x, y, c=which, s=2, cmap="coolwarm", alpha=0.28, linewidths=0)
ax.set_title("700,000 points")
ax.set_xlabel("x")
ax.set_ylabel("y")

plt.show()
