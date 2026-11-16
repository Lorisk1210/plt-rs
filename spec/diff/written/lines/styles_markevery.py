# Several model curves with distinct line styles, widths and markevery so markers do not clutter.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(0, 10, 500)
fig, ax = plt.subplots(figsize=(7, 4))
ax.plot(x, np.sqrt(x), ls="-", lw=2, marker="o", markevery=50, label="sqrt")
ax.plot(x, np.log1p(x), ls="--", lw=1.5, marker="s", markevery=(25, 50), label="log1p")
ax.plot(x, x / 4, ls=":", lw=2.5, marker="^", markevery=0.1, label="linear")
ax.plot(x, 1 - np.exp(-x / 2), ls="-.", color="k", label="saturation")
ax.plot(x, 0.2 * np.sin(x) + 1, ls=(0, (5, 2, 1, 2)), color="gray", label="custom dash")
ax.legend()
print([l.get_linestyle() for l in ax.lines])
fig.savefig("models.png")
