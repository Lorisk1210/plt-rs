# Lambert azimuthal projection centred on a pole-ish point to show a polar cap region.
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(5, 5))
ax = fig.add_subplot(projection="lambert", center_longitude=0, center_latitude=np.pi / 3)
lon = np.linspace(-np.pi, np.pi, 100)
for lat in np.deg2rad([60, 70, 80]):
    ax.plot(lon, np.full_like(lon, lat), lw=1)
ax.fill(lon, np.full_like(lon, np.deg2rad(75)), alpha=0.3, color="lightblue")
ax.grid(True)
ax.set_title("polar cap")
fig.savefig("cap.png")
print(ax.name)
