# Two heat maps side by side, each with its own colour bar and label.
import numpy as np
import matplotlib.pyplot as plt

y, x = np.mgrid[0:1:50j, 0:1:50j]
temperature = 20 + 10 * np.sin(np.pi * x) * np.cos(np.pi * y)
pressure = 1000 + 30 * x * y
fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(9, 3.8))
im1 = ax1.imshow(temperature, origin="lower", cmap="inferno")
cb1 = fig.colorbar(im1, ax=ax1)
cb1.set_label("temperature [°C]")
im2 = ax2.imshow(pressure, origin="lower", cmap="Blues")
cb2 = plt.colorbar(im2, ax=ax2, fraction=0.046, pad=0.04)
cb2.set_label("pressure [hPa]", rotation=270, labelpad=15)
ax1.set_title("temperature")
ax2.set_title("pressure")
print(cb1.vmin, cb1.vmax, cb2.orientation)
fig.savefig("two_bars.png")
