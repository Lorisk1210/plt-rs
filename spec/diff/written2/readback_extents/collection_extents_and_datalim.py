# get_datalim and get_window_extent of collections with offsets in different transforms; ax.dataLim after each addition.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.collections import CircleCollection, PolyCollection, LineCollection

fig, ax = plt.subplots(figsize=(4, 4), dpi=72)
r = fig.canvas.get_renderer()


def report(tag, coll):
    fig.canvas.draw()
    print(tag, [round(v, 3) for v in coll.get_datalim(ax.transData).bounds],
          [round(v, 1) for v in coll.get_window_extent(r).bounds], [round(v, 3) for v in ax.dataLim.bounds])


sc = ax.scatter([1, 2, 3], [1, 4, 9])
report("scatter", sc)
cc = CircleCollection([100, 400], offsets=[(5, 5), (6, 6)], offset_transform=ax.transData)
ax.add_collection(cc)
report("circles", cc)
pc = PolyCollection([[(0, 0), (1, 0), (0, 1)], [(10, 10), (12, 10), (11, 12)]])
ax.add_collection(pc)
report("polys", pc)
lc = LineCollection([[(0, 0), (0.5, 0.5)]], offsets=[(20, 0)], offset_transform=ax.transData)
ax.add_collection(lc)
report("offset lines", lc)
ax_pc = PolyCollection([[(0.1, 0.1), (0.2, 0.1), (0.2, 0.2)]], transform=ax.transAxes)
ax.add_collection(ax_pc)
report("axes coords", ax_pc)
ax.autoscale_view()
fig.canvas.draw()
print([round(v, 3) for v in ax.get_xlim()], [round(v, 3) for v in ax.get_ylim()])
ax.update_datalim([(30, 30)])
print([round(v, 3) for v in ax.dataLim.bounds])
ax.relim()
print([round(v, 3) for v in ax.dataLim.bounds])
ax.relim(visible_only=True)
print([round(v, 3) for v in ax.dataLim.bounds], ax.ignore_existing_data_limits)
fig.savefig("collections_lim.png")
