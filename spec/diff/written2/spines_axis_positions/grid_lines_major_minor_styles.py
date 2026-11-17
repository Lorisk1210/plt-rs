# Grid lines: which major/minor/both per axis, styles via grid() kwargs and rcParams, axisbelow, and gridline objects read back.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3), dpi=60)
ax.set_xlim(0, 10)
ax.minorticks_on()
ax.grid(True, which="major", axis="x", color="r", linestyle="--", linewidth=1.5)
ax.grid(True, which="minor", axis="y", color="0.8", linestyle=":", alpha=0.6)
fig.canvas.draw()
mx = ax.xaxis.get_major_ticks()[1].gridline
my = ax.yaxis.get_minor_ticks()[1].gridline
print(mx.get_visible(), mx.get_color(), mx.get_linestyle(), mx.get_linewidth())
print(my.get_visible(), my.get_color(), my.get_linestyle(), my.get_alpha())
print(ax.yaxis.get_major_ticks()[1].gridline.get_visible(), ax.xaxis.get_minor_ticks()[1].gridline.get_visible())
print(len(ax.get_xgridlines()), len(ax.get_ygridlines()))
ax.grid(False)
fig.canvas.draw()
print(ax.xaxis.get_major_ticks()[1].gridline.get_visible(), ax.yaxis.get_minor_ticks()[1].gridline.get_visible())
ax.grid(which="both")
fig.canvas.draw()
print(ax.xaxis.get_minor_ticks()[1].gridline.get_visible(), ax.xaxis.get_minor_ticks()[1].gridline.get_color())
with plt.rc_context({"axes.grid": True, "axes.grid.which": "minor", "axes.grid.axis": "y", "grid.color": "b",
                     "grid.linewidth": 2, "xtick.minor.visible": True, "ytick.minor.visible": True}):
    f2, a2 = plt.subplots()
    f2.canvas.draw()
    print(a2.yaxis.get_minor_ticks()[0].gridline.get_visible(), a2.yaxis.get_major_ticks()[0].gridline.get_visible(),
          a2.yaxis.get_minor_ticks()[0].gridline.get_color(), a2.yaxis.get_minor_ticks()[0].gridline.get_linewidth())
ax.set_axisbelow(True)
print(ax.get_axisbelow(), ax.xaxis.get_zorder())
fig.savefig("grids.png")
