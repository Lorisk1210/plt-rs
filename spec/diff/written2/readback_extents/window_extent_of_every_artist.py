# get_window_extent of every artist in a busy figure, walked with findobj; grouped by type with rounded bounds.
import collections
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, (a1, a2) = plt.subplots(1, 2, figsize=(7, 3), dpi=80)
a1.plot(np.arange(5), np.arange(5) ** 2, "o-", label="sq")
a1.bar([1, 2], [3, 4], label="bars")
a1.legend()
a1.set_title("left")
a1.annotate("note", (2, 4), (3, 10), arrowprops=dict(arrowstyle="->"))
a2.scatter([0, 1, 2], [2, 1, 0], s=[10, 50, 100])
a2.imshow(np.eye(3), extent=(0, 2, 0, 2))
fig.suptitle("extents")
fig.canvas.draw()
r = fig.canvas.get_renderer()
by_type = collections.defaultdict(list)
for art in fig.findobj():
    try:
        bb = art.get_window_extent(r)
    except Exception as e:
        by_type[type(art).__name__].append(type(e).__name__)
        continue
    by_type[type(art).__name__].append(tuple(round(v, 1) for v in bb.bounds))
for name in sorted(by_type):
    vals = by_type[name]
    print(name, len(vals), vals[:2])
print(len(fig.findobj()), len(a1.findobj()), len(fig.findobj(include_self=False)))
hi = fig.suptitle("extents").get_window_extent(dpi=200)
print([round(v, 1) for v in hi.bounds], [round(v, 1) for v in fig.bbox.bounds])
fig.savefig("extents.png")
