# A 3x3 GridSpec where one plot spans two rows and another spans two columns via slicing.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.gridspec as gridspec

fig = plt.figure(figsize=(8, 6), layout="constrained")
gs = gridspec.GridSpec(3, 3, figure=fig)
big = fig.add_subplot(gs[:2, :2])
side = fig.add_subplot(gs[:2, 2])
bottom = fig.add_subplot(gs[2, :])

t = np.linspace(0, 1, 200)
big.plot(t, np.sin(12 * t) * t, lw=2)
big.set_title("main")
side.barh(["a", "b", "c"], [5, 2, 7])
side.set_title("side")
bottom.fill_between(t, 0, np.cos(6 * t) ** 2, color="lightgreen")
bottom.set_title("bottom strip")
for ax in fig.axes:
    ss = ax.get_subplotspec()
    print(ss.rowspan, ss.colspan)
fig.savefig("slices.png")
