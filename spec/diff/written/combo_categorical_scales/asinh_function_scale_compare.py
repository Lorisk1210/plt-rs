# The same data on linear, log, symlog, asinh and a custom function scale in a row of panels, ticks printed.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-50, 50, 300)
y = x ** 3 / 100
fig, axs = plt.subplots(1, 5, figsize=(16, 3.5))
scales = [("linear", {}), ("log", {"nonpositive": "mask"}), ("symlog", {"linthresh": 1}), ("asinh", {"linear_width": 2}),
          ("function", {"functions": (np.cbrt, lambda v: v ** 3)})]
for ax, (name, kw) in zip(axs, scales):
    ax.plot(x, y)
    ax.set_yscale(name, **kw)
    ax.set_title(name)
fig.tight_layout()
fig.canvas.draw()
for ax in axs:
    print(ax.get_yscale(), [t.get_text() for t in ax.get_yticklabels()][:4])
fig.savefig("scales_row.png", dpi=40)
