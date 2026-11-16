# Customise longitude tick labels on a Mollweide plot (hours like an astronomer) and grid spacing.
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(8, 4.5))
ax = fig.add_subplot(projection="mollweide")
ra = np.deg2rad(np.array([10, 45, 120, 200, 300]) - 180)
dec = np.deg2rad([20, -40, 60, 5, -10])
ax.scatter(ra, dec, s=60, c="gold", edgecolors="k", zorder=3)
ax.set_xticks(np.deg2rad(np.arange(-150, 151, 30)))
ax.set_xticklabels([f"{h}h" for h in range(14, 24, 2)] + [f"{h}h" for h in range(0, 12, 2)], fontsize=7)
ax.set_longitude_grid(30)
ax.set_latitude_grid(15)
ax.grid(True)
print(len(ax.get_xticks()))
fig.savefig("sky_hours.png")
