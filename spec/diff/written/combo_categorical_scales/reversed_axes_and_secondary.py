# Ocean profile: depth on an inverted y, temperature on bottom x, salinity on a twiny top x, density on a secondary y.
import numpy as np
import matplotlib.pyplot as plt

depth = np.linspace(0, 1000, 100)
temp = 4 + 18 * np.exp(-depth / 150)
sal = 34.5 + 0.8 * (1 - np.exp(-depth / 300))
fig, ax = plt.subplots(figsize=(5, 6))
ax.plot(temp, depth, "C3")
ax.invert_yaxis()
ax.set_xlabel("temperature [°C]", color="C3")
ax.set_ylabel("depth [m]")
top = ax.twiny()
top.plot(sal, depth, "C0")
top.set_xlabel("salinity [psu]", color="C0")
sec = ax.secondary_yaxis("right", functions=(lambda d: d * 1.025 / 10, lambda p: p * 10 / 1.025))
sec.set_ylabel("pressure [dbar]")
fig.tight_layout()
print(ax.yaxis_inverted(), top.yaxis_inverted(), [round(v, 3) for v in top.get_xlim()])
fig.savefig("profile.png", dpi=50)
