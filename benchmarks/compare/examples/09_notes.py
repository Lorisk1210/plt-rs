"""A curve with many labels

A damped sine with 2,800 small text annotations along it
(``Axes.annotate``).
"""

import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(11)
x = np.linspace(0.0, 8.0, 500)
y = np.sin(x) * np.exp(-0.18 * x)

fig, ax = plt.subplots(figsize=(6.6, 4.3), dpi=100)
ax.plot(x, y)
for i in range(2_800):
    xi = float(rng.uniform(0.15, 7.85))
    yi = float(np.sin(xi) * np.exp(-0.18 * xi) + rng.normal(scale=0.04))
    ax.annotate(str(i % 100), (xi, yi), fontsize=6)
ax.set_title("2,800 labels")
ax.set_xlabel("time (s)")
ax.set_ylabel("amplitude")

plt.show()
