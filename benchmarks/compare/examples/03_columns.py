"""Ten thousand bars

10,000 bars of a rectified sine (``Axes.bar``).
"""

import numpy as np
import matplotlib.pyplot as plt

n = 10_000
x = np.arange(n)
y = np.abs(np.sin(x / 23.0)) + 0.12

fig, ax = plt.subplots(figsize=(6.6, 4.3), dpi=100)
ax.bar(x, y, width=1.0)
ax.set_title("10,000 bars")
ax.set_xlabel("bin")
ax.set_ylabel("count")

plt.show()
