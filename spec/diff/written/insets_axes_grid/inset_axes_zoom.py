# Zoomed inset of a noisy peak with ax.inset_axes and indicate_inset_zoom connector lines.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 10, 2000)
y = np.exp(-(x - 6) ** 2 * 20) + 0.05 * np.sin(40 * x)
fig, ax = plt.subplots(figsize=(7, 4))
ax.plot(x, y, lw=0.8)
axins = ax.inset_axes([0.05, 0.45, 0.4, 0.45], xlim=(5.6, 6.4), ylim=(0.6, 1.1))
axins.plot(x, y, lw=0.8)
axins.set_xticklabels([])
axins.set_yticklabels([])
rect, connectors = ax.indicate_inset_zoom(axins, edgecolor="black")
print(len(connectors), axins.get_xlim())
fig.savefig("zoom.png")
