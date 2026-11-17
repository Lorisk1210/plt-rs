# tick_params combinations (direction, length, width, colors, pad, labelsize, which, top/right labels, grid params) read back.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3), dpi=60)
ax.tick_params(axis="both", which="major", direction="inout", length=8, width=2, colors="r", pad=6, labelsize=7,
               grid_color="0.7", grid_linestyle=":", grid_alpha=0.5)
ax.tick_params(axis="x", which="minor", direction="in", length=3, bottom=True, top=True)
ax.tick_params(axis="y", labelleft=False, labelright=True, right=True, left=False, labelrotation=45)
ax.minorticks_on()
ax.grid(True)
fig.canvas.draw()
t = ax.xaxis.get_major_ticks()[1]
print(t._tickdir, round(t.tick1line.get_markersize(), 2), t.tick1line.get_markeredgewidth(), t.tick1line.get_color(),
      t.get_pad(), t.label1.get_fontsize(), t.label1.get_color())
print(t.gridline.get_color(), t.gridline.get_linestyle(), t.gridline.get_alpha(), t.gridline.get_visible())
yt = ax.yaxis.get_major_ticks()[1]
print(yt.tick1line.get_visible(), yt.tick2line.get_visible(), yt.label1.get_visible(), yt.label2.get_visible(),
      yt.label2.get_rotation())
mt = ax.xaxis.get_minor_ticks()[0]
print(mt._tickdir, round(mt.tick1line.get_markersize(), 2), mt.tick2line.get_visible())
print(sorted(ax.xaxis._major_tick_kw)[:6])
ax.tick_params(reset=True, axis="x")
fig.canvas.draw()
t = ax.xaxis.get_major_ticks()[1]
print(t._tickdir, round(t.tick1line.get_markersize(), 2), t.label1.get_color())
try:
    ax.tick_params(axis="x", lenght=5)
except ValueError as e:
    print("ValueError", str(e)[:60])
fig.savefig("tick_params.png")
