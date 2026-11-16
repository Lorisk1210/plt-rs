# Set exactly the ticks and their labels (set_xticks with labels) for angles in radians.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 2 * np.pi, 200)
fig, ax = plt.subplots(figsize=(6, 3.5))
ax.plot(x, np.sin(x), label="sin")
ax.plot(x, np.cos(x), label="cos")
ax.set_xticks([0, np.pi / 2, np.pi, 3 * np.pi / 2, 2 * np.pi])
ax.set_xticklabels(["0", r"$\pi/2$", r"$\pi$", r"$3\pi/2$", r"$2\pi$"])
ax.set_yticks([-1, 0, 1])
ax.set_yticks([-0.5, 0.5], minor=True)
ax.tick_params(which="minor", length=3)
ax.legend(loc="lower left")
print(ax.get_xticks().round(3), [t.get_text() for t in ax.get_xticklabels()])
fig.savefig("radians.png")
