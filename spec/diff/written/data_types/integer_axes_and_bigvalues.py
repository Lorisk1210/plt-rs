# Very large and very small magnitudes: offsets, scientific labels, and integer years as x.
import numpy as np
import matplotlib.pyplot as plt

years = np.arange(1990, 2025)
fig, axs = plt.subplots(1, 3, figsize=(11, 3))
axs[0].plot(years, np.exp((years - 1990) / 10) * 1e9)
axs[0].set_title("1e9 scale")
axs[1].plot(years, 1e-7 * np.sin(years))
axs[1].set_title("1e-7 scale")
axs[2].plot(years, 123456789 + np.arange(35))
axs[2].set_title("offset")
fig.tight_layout()
fig.canvas.draw()
print([ax.yaxis.get_offset_text().get_text() for ax in axs])
fig.savefig("magnitudes.png")
