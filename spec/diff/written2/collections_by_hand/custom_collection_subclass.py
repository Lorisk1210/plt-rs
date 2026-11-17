# Subclass Collection: a "dumbbell" collection drawing pairs of points joined by bars, with its own draw using draw_path_collection.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.collections import Collection
from matplotlib.path import Path


class Dumbbells(Collection):
    def __init__(self, starts, ends, **kw):
        super().__init__(**kw)
        self.starts = np.asarray(starts, float)
        self.ends = np.asarray(ends, float)
        self._paths = [Path([s, e]) for s, e in zip(self.starts, self.ends)]

    def get_datalim(self, transData):
        from matplotlib.transforms import Bbox
        pts = np.vstack([self.starts, self.ends])
        return Bbox([pts.min(axis=0), pts.max(axis=0)])

    def draw(self, renderer):
        self.set_transform(self.axes.transData)
        super().draw(renderer)


fig, ax = plt.subplots(figsize=(4, 3))
db = Dumbbells([(0, 0), (1, 2), (3, 1)], [(2, 0), (2, 3), (4, 4)], edgecolors=["r", "g", "b"], linewidths=3,
               facecolors="none")
ax.add_collection(db)
ax.scatter(*np.vstack([db.starts, db.ends]).T, zorder=3)
ax.autoscale_view()
fig.canvas.draw()
print(len(db.get_paths()), [round(v, 3) for v in ax.get_xlim()], [round(v, 3) for v in ax.get_ylim()])
print(np.round(db.get_edgecolor(), 3).tolist(), db.get_linewidth().tolist())
print([round(v, 1) for v in db.get_window_extent(fig.canvas.get_renderer()).bounds])
db.set_linestyle(":")
db.set_alpha(0.5)
fig.savefig("dumbbells.png")
print(db.get_alpha(), isinstance(db, Collection), db.get_offsets().shape)
