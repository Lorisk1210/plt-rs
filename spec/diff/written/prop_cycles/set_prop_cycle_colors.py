# Set a custom colour and linestyle cycle on one axes with set_prop_cycle and cycler addition.
import numpy as np
import matplotlib.pyplot as plt
from cycler import cycler

x = np.linspace(0, 2 * np.pi, 50)
fig, ax = plt.subplots(figsize=(6, 4))
ax.set_prop_cycle(cycler(color=["c", "m", "y", "k"]) + cycler(linestyle=["-", "--", ":", "-."]))
for k in range(6):
    ax.plot(x, np.sin(x + k * np.pi / 6) + k * 0.3, label=f"k={k}")
ax.legend(fontsize=8)
print([(l.get_color(), l.get_linestyle()) for l in ax.lines])
fig.savefig("cycle.png")
