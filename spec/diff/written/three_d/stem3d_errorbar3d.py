# 3-D stem plot of a sampled spiral and 3-D error bars for a few points.
import numpy as np
import matplotlib.pyplot as plt

t = np.linspace(0, 2 * np.pi, 20)
fig = plt.figure(figsize=(9, 4))
ax1 = fig.add_subplot(1, 2, 1, projection="3d")
ax1.stem(np.cos(t), np.sin(t), t)
ax1.set_title("stem")
ax2 = fig.add_subplot(1, 2, 2, projection="3d")
ax2.errorbar([0, 1, 2], [1, 2, 1], [2, 3, 1], zerr=[0.2, 0.3, 0.1], fmt="o", capsize=3)
ax2.set_title("errorbar")
print(len(ax1.collections), len(ax2.lines))
fig.savefig("stem3d.png")
