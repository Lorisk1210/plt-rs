"""Decaying chirp, filled steps

A chirp whose amplitude decays, drawn as 180,000 filled steps
(``Axes.stairs`` with ``fill=True``).
"""

import numpy as np
import matplotlib.pyplot as plt

n = 180_000
t = np.linspace(0.0, 8.0, n)
y = np.exp(-0.22 * t) * np.sin(2 * np.pi * (1.5 * t + 0.55 * t * t))

fig, ax = plt.subplots(figsize=(6.6, 4.3), dpi=100)
ax.stairs(y, np.linspace(t[0], t[-1], n + 1), fill=True)
ax.set_xlim(t[0], t[-1])
ax.set_title("180,000 filled steps")
ax.set_xlabel("time (s)")
ax.set_ylabel("amplitude")

plt.show()
