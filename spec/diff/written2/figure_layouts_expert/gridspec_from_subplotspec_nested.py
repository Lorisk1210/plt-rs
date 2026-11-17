# Nested GridSpecFromSubplotSpec three levels deep, with spans, ratios and subplot_kw; positions and tight_layout effects.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.gridspec import GridSpec, GridSpecFromSubplotSpec

fig = plt.figure(figsize=(6, 5), dpi=60)
outer = GridSpec(1, 2, figure=fig, width_ratios=[2, 1], wspace=0.2)
left = GridSpecFromSubplotSpec(2, 2, subplot_spec=outer[0], hspace=0.4, height_ratios=[1, 3])
right = GridSpecFromSubplotSpec(3, 1, subplot_spec=outer[1])
inner = GridSpecFromSubplotSpec(1, 2, subplot_spec=left[1, :], wspace=0.05)
axes = {
    "top-left": fig.add_subplot(left[0, 0]),
    "top-right": fig.add_subplot(left[0, 1]),
    "inner-a": fig.add_subplot(inner[0]),
    "inner-b": fig.add_subplot(inner[1]),
    "r0": fig.add_subplot(right[0]),
    "r12": fig.add_subplot(right[1:, 0], projection="polar"),
}
fig.canvas.draw()
for name, ax in axes.items():
    print(name, [round(v, 4) for v in ax.get_position().bounds])
ss = axes["inner-a"].get_subplotspec()
print(ss.get_gridspec().get_geometry(), ss.get_topmost_subplotspec().get_gridspec() is outer)
print(type(ss.get_gridspec()).__name__, ss.get_gridspec().get_subplot_params(fig).wspace)
fig.tight_layout()
print([round(v, 4) for v in axes["r12"].get_position().bounds])
print(outer.get_width_ratios(), left.get_height_ratios(), right.nrows)
sub = outer[1].subgridspec(2, 2)
print(type(sub).__name__, sub.get_geometry())
fig.savefig("nested.png")
