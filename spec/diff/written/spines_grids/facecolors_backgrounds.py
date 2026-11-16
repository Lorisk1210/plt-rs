# Axes and figure background colours, alternating band shading, and a gradient background image.
import numpy as np
import matplotlib.pyplot as plt

fig, axs = plt.subplots(1, 3, figsize=(10, 3), facecolor="#f4f1ea")
x = np.linspace(0, 10, 50)
axs[0].set_facecolor("#eaeaf2")
axs[0].plot(x, np.sin(x), color="#4c72b0")
axs[0].grid(color="white")
for k in range(0, 10, 2):
    axs[1].axvspan(k, k + 1, color="0.92", zorder=0)
axs[1].plot(x, np.cos(x))
axs[2].imshow([[0, 0], [1, 1]], cmap="Blues", extent=[0, 10, -1, 1], aspect="auto", alpha=0.4, interpolation="bicubic")
axs[2].plot(x, np.sin(2 * x), color="k")
print(axs[0].get_facecolor())
fig.savefig("backgrounds.png", facecolor=fig.get_facecolor())
