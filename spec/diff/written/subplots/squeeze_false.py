# Write a helper that plots any number of columns; squeeze=False keeps a 2-D array even for one panel.
import numpy as np
import matplotlib.pyplot as plt


def plot_columns(data, names):
    fig, axes = plt.subplots(1, len(names), squeeze=False, figsize=(3 * len(names), 2.5))
    for ax, name, col in zip(axes[0], names, data.T):
        ax.plot(col)
        ax.set_title(name)
    return fig, axes


data = np.cumsum(np.ones((30, 3)) * [1, 2, -1], axis=0)
fig1, axes1 = plot_columns(data[:, :1], ["only"])
fig2, axes2 = plot_columns(data, ["up", "faster", "down"])
print(axes1.shape, axes2.shape)
fig1.savefig("one_column.png")
fig2.savefig("three_columns.png")
