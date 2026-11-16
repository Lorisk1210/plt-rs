# Draw implicit curves (circle, lemniscate, heart) as the zero contour of a function.
import numpy as np
import matplotlib.pyplot as plt

x, y = np.meshgrid(np.linspace(-2, 2, 400), np.linspace(-2, 2, 400))
curves = {"circle": x ** 2 + y ** 2 - 1, "lemniscate": (x ** 2 + y ** 2) ** 2 - 2 * (x ** 2 - y ** 2),
          "heart": (x ** 2 + y ** 2 - 1) ** 3 - x ** 2 * y ** 3}
fig, ax = plt.subplots(figsize=(5, 5))
for (name, f), c in zip(curves.items(), ["C0", "C1", "C3"]):
    cs = ax.contour(x, y, f, levels=[0], colors=c)
    ax.plot([], [], color=c, label=name)
ax.set_aspect("equal")
ax.legend(loc="lower right")
ax.grid(alpha=0.3)
print(len(ax.collections) if hasattr(ax, "collections") else 0)
fig.savefig("implicit.png")
