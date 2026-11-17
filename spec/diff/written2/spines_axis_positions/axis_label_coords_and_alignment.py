# set_label_coords, align_xlabels/align_ylabels/align_labels across subplots with different tick widths; label positions.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, axs = plt.subplots(2, 2, figsize=(6, 5), dpi=60)
axs[0, 0].set_ylim(0, 1)
axs[1, 0].set_ylim(0, 1e6)
axs[0, 1].set_xlim(0, 1)
axs[1, 1].tick_params(axis="x", labelrotation=55)
for a in axs.flat:
    a.set_xlabel("xlabel")
    a.set_ylabel("ylabel")
fig.canvas.draw()
r = fig.canvas.get_renderer()


def label_x(a):
    return round(a.yaxis.label.get_window_extent(r).x0, 1)


def label_y(a):
    return round(a.xaxis.label.get_window_extent(r).y0, 1)


print([label_x(a) for a in axs[:, 0]], [label_y(a) for a in axs[1, :]])
fig.align_ylabels(axs[:, 0])
fig.align_xlabels(axs[1, :])
fig.canvas.draw()
print([label_x(a) for a in axs[:, 0]], [label_y(a) for a in axs[1, :]])
axs[0, 1].yaxis.set_label_coords(-0.3, 0.5)
axs[0, 1].xaxis.set_label_coords(0.5, -0.4, transform=axs[0, 1].transAxes)
fig.canvas.draw()
print(label_x(axs[0, 1]), label_y(axs[0, 1]), axs[0, 1].yaxis._autolabelpos)
fig.align_labels()
fig.canvas.draw()
print([label_x(a) for a in axs.flat])
print(sorted(fig._align_label_groups))
fig.savefig("aligned.png")
