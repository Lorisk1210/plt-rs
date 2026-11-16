# Two groups of panels, each with its own inner grid, using GridSpecFromSubplotSpec.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.gridspec import GridSpec, GridSpecFromSubplotSpec

fig = plt.figure(figsize=(9, 4))
outer = GridSpec(1, 2, figure=fig, wspace=0.25)
left = GridSpecFromSubplotSpec(2, 2, subplot_spec=outer[0], hspace=0.4)
right = GridSpecFromSubplotSpec(3, 1, subplot_spec=outer[1], hspace=0.6)
x = np.linspace(0, 1, 50)
for i in range(4):
    ax = fig.add_subplot(left[i // 2, i % 2])
    ax.plot(x, x ** (i + 1))
    ax.set_title(f"power {i + 1}", fontsize=8)
for j in range(3):
    ax = fig.add_subplot(right[j])
    ax.plot(x, np.sin(2 * np.pi * (j + 1) * x), color="C3")
    ax.set_yticks([])
print(len(fig.axes), left.get_geometry(), right.get_geometry())
fig.savefig("nested.png")
