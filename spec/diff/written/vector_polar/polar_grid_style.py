# Restyle a polar grid: theta gridline spacing, radial label angle, grid colours and log r.
import numpy as np
import matplotlib.pyplot as plt

theta = np.linspace(0, 6 * np.pi, 500)
r = np.exp(theta / 6)
fig = plt.figure(figsize=(5, 5))
ax = fig.add_subplot(111, polar=True)
ax.plot(theta, r, color="darkgreen")
ax.set_rscale("log")
ax.set_thetagrids(range(0, 360, 30))
ax.grid(color="0.6", ls=":")
ax.tick_params(axis="y", labelsize=7, colors="gray")
ax.set_title("log spiral")
print(ax.get_yscale(), len(ax.xaxis.get_ticklocs()))
fig.savefig("spiral.png")
