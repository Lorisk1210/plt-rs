# Compute a figure size from the number of panels so each subplot stays 3 x 2.5 inches.
import numpy as np
import matplotlib.pyplot as plt

n_panels = 7
ncols = 3
nrows = int(np.ceil(n_panels / ncols))
fig, axes = plt.subplots(nrows, ncols, figsize=(3 * ncols, 2.5 * nrows))
x = np.linspace(0, 1, 50)
for i, ax in enumerate(axes.flat):
    if i < n_panels:
        ax.plot(x, x ** (i + 1))
        ax.set_title(f"x^{i + 1}")
    else:
        ax.set_visible(False)
fig.tight_layout()
print(axes.shape, fig.get_size_inches())
fig.savefig("panels.png", dpi=60)
