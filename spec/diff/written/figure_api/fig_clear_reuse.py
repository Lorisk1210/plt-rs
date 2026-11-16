# Reuse one Figure object for several plots: clear(keep_observers), change layout, save each.
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(5, 3.5))
for k, kind in enumerate(["line", "bar", "scatter"]):
    fig.clear()
    ax = fig.add_subplot()
    x = np.arange(6)
    getattr(ax, {"line": "plot", "bar": "bar", "scatter": "scatter"}[kind])(x, x ** 1.5)
    ax.set_title(kind)
    fig.savefig(f"reuse_{kind}.png")
    print(kind, len(fig.axes), len(ax.get_children()) > 0)
