# twinx/twiny chains: shared limits, autoscaling independence, draw order of twins, patch visibility and label positions.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3), dpi=60)
ax.plot([0, 10], [0, 1], "C0")
t1 = ax.twinx()
t1.plot([0, 20], [100, 300], "C1")
t2 = ax.twiny()
t2.plot([1000, 2000], [0.5, 0.6], "C2")
t3 = t1.twiny()
fig.canvas.draw()
for a in (ax, t1, t2, t3):
    print([round(v, 2) for v in a.get_xlim()], [round(v, 2) for v in a.get_ylim()], a.patch.get_visible(),
          a.yaxis.get_label_position(), a.xaxis.get_label_position())
print(ax.get_shared_x_axes().joined(ax, t1), ax.get_shared_y_axes().joined(ax, t2), t1.get_shared_x_axes().joined(t1, t3))
print([fig.axes.index(a) for a in (ax, t1, t2, t3)], [a.get_zorder() for a in fig.axes])
ax.set_xlim(0, 5)
print([round(v, 2) for v in t1.get_xlim()], [round(v, 2) for v in t2.get_xlim()])
t1.set_zorder(-1)
ax.patch.set_visible(False)
fig.canvas.draw()
print(sorted(a.get_zorder() for a in fig.axes))
t1.remove()
print(len(fig.axes), ax.get_shared_x_axes().get_siblings(ax) == [ax] or len(ax.get_shared_x_axes().get_siblings(ax)))
fig.savefig("twins.png")
t4 = ax.twinx()
t4.set_ylabel("new twin")
fig.canvas.draw()
print(t4.yaxis.get_label_position(), [round(v, 2) for v in t4.get_ylim()], len(fig.axes))
