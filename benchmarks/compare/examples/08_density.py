"""Joint density

A 240 × 240 two-dimensional histogram of 5,000,000 correlated samples
(``Axes.hist2d``).
"""

import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(3)
n = 5_000_000
x = rng.normal(size=n)
y = 0.65 * x + rng.normal(scale=0.55, size=n)

fig, ax = plt.subplots(figsize=(6.6, 4.3), dpi=100)
ax.hist2d(x, y, bins=240, cmap="magma")
ax.set_title("5,000,000 samples")
ax.set_xlabel("x")
ax.set_ylabel("y")

plt.show()
