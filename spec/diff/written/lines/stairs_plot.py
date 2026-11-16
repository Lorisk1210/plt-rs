# Binned rates drawn as stairs, with a baseline and a filled version stacked.
import numpy as np
import matplotlib.pyplot as plt

edges = np.array([0, 10, 20, 35, 50, 70, 100])
low = np.array([5, 8, 12, 9, 6, 2])
high = np.array([2, 4, 6, 8, 5, 3])
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.stairs(low, edges, fill=True, label="low", alpha=0.6)
ax.stairs(low + high, edges, baseline=low, fill=True, label="high", alpha=0.6)
ax.stairs(low + high, edges, color="k", lw=1.5, label="total")
ax.legend()
print(type(ax.patches[0]).__name__)
fig.savefig("stairs.png")
