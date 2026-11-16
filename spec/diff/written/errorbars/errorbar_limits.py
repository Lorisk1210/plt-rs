# Upper and lower limits drawn with errorbar uplims/lolims arrows for non-detections.
import numpy as np
import matplotlib.pyplot as plt

x = np.arange(1, 9)
y = np.array([3.0, 2.5, 4.1, 3.3, 2.0, 1.5, 3.8, 4.4])
err = np.full(8, 0.4)
upper = np.array([0, 0, 0, 1, 1, 0, 0, 0], dtype=bool)
lower = np.array([0, 0, 0, 0, 0, 1, 0, 0], dtype=bool)
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.errorbar(x, y, yerr=err, uplims=upper, lolims=lower, fmt="ks", capsize=3, mfc="white")
ax.set_xlabel("source")
ax.set_ylabel("flux")
ax.set_title("detections and limits")
ax.set_xlim(0, 9)
print(len(ax.lines), len(ax.collections))
fig.savefig("limits.png")
