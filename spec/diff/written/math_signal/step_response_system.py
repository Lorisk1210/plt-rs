# Step responses of damped oscillators for several damping ratios with overshoot markers.
import numpy as np
import matplotlib.pyplot as plt

t = np.linspace(0, 10, 500)
w = 2.0
fig, ax = plt.subplots(figsize=(7, 4))
for zeta in (0.1, 0.3, 0.7, 1.0):
    if zeta < 1:
        wd = w * np.sqrt(1 - zeta ** 2)
        y = 1 - np.exp(-zeta * w * t) * (np.cos(wd * t) + zeta / np.sqrt(1 - zeta ** 2) * np.sin(wd * t))
    else:
        y = 1 - np.exp(-w * t) * (1 + w * t)
    line, = ax.plot(t, y, label=f"ζ={zeta}")
    ax.plot(t[y.argmax()], y.max(), "o", color=line.get_color())
ax.axhline(1, color="k", ls=":")
ax.legend()
ax.set_xlabel("time [s]")
print(ax.get_ylim())
fig.savefig("steps.png")
