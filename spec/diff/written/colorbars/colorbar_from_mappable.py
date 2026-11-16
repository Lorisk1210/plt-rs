# Stand-alone colour bar for a colour-coded line plot via ScalarMappable (no image in the axes).
import numpy as np
import matplotlib.pyplot as plt
import matplotlib as mpl

speeds = np.linspace(10, 50, 9)
cmap = plt.cm.viridis
norm = mpl.colors.Normalize(vmin=speeds.min(), vmax=speeds.max())
fig, ax = plt.subplots(figsize=(6, 4))
t = np.linspace(0, 2, 100)
for v in speeds:
    ax.plot(t, v * t - 4.9 * t ** 2, color=cmap(norm(v)))
sm = mpl.cm.ScalarMappable(norm=norm, cmap=cmap)
cb = fig.colorbar(sm, ax=ax, label="launch speed [m/s]")
ax.set_ylim(bottom=0)
ax.set_xlabel("time [s]")
ax.set_ylabel("height [m]")
print(sm.get_array(), cb.vmax)
fig.savefig("launches.png")
