"""Six random walks

Six random walks of 35,000 steps each, drawn as filled steps in a 2 × 3
grid laid out by the constrained layout engine.
"""

import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(5)
n = 35_000

fig, axs = plt.subplots(2, 3, figsize=(7.0, 4.6), dpi=100, layout="constrained", squeeze=False)
for ax in axs.flat:
    y = np.cumsum(rng.normal(size=n))
    y /= np.max(np.abs(y))
    ax.stairs(y, fill=True)
    ax.set_xticks([])
    ax.set_yticks([])
fig.suptitle("6 walks, 35,000 steps each")

plt.show()
