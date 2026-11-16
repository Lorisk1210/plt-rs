# Cycle line width and alpha (not only colour) to show older series fading.
import numpy as np
import matplotlib.pyplot as plt
from cycler import cycler

ages = np.arange(6)
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.set_prop_cycle(cycler(alpha=1 - ages / 7) + cycler(linewidth=3 - ages * 0.4) + cycler(color=["k"] * 6))
x = np.linspace(0, 1, 60)
for k in ages:
    ax.plot(x, np.sin(2 * np.pi * (x - k * 0.05)))
print([(round(l.get_alpha(), 3), round(l.get_linewidth(), 2)) for l in ax.lines])
fig.savefig("fading.png")
