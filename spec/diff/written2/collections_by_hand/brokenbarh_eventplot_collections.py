# broken_barh and eventplot as the collections they are: segments, offsets, orientation changes and lineoffsets/linelengths.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, (a1, a2) = plt.subplots(2, 1, figsize=(5, 4))
bb = a1.broken_barh([(1, 2), (5, 1.5), (8, 0.5)], (0, 1), facecolors=["C0", "C1", "C2"])
bb2 = a1.broken_barh([(0, 10)], (1.5, 0.5), facecolors="0.8", align="center")
fig.canvas.draw()
print(type(bb).__name__, len(bb.get_paths()), np.round(bb.get_paths()[1].vertices[:2], 3).tolist())
print([round(v, 3) for v in a1.get_xlim()], [round(v, 3) for v in a1.get_ylim()], np.round(bb2.get_paths()[0].get_extents().bounds, 3).tolist())
events = [np.array([1, 2, 5]), np.array([0.5, 3]), np.array([])]
ev = a2.eventplot(events, lineoffsets=[0, 2, 4], linelengths=[1, 0.5, 2], colors=["r", "g", "b"])
print([type(e).__name__ for e in ev], [np.asarray(e.get_positions()).tolist() for e in ev])
print([e.get_lineoffset() for e in ev], [e.get_linelength() for e in ev], [e.get_orientation() for e in ev])
ev[0].switch_orientation()
ev[1].set_positions([4, 4.5])
ev[1].add_positions([6])
ev[2].extend_positions([1, 2])
fig.canvas.draw()
print(ev[0].get_orientation(), ev[0].is_horizontal(), np.asarray(ev[1].get_positions()).tolist(), np.asarray(ev[2].get_positions()).tolist())
print(np.round(ev[0].get_segments()[0], 3).tolist())
ev[1].set_lineoffset(-1)
ev[1].set_linelength(3)
print(np.round(ev[1].get_segments()[0], 3).tolist())
fig.savefig("events.png")
bb.set_verts([[(0, 0), (1, 0), (1, 1), (0, 1)]])
print(len(bb.get_paths()), type(ev[0]).__mro__[1].__name__)
print(len(ax2_list := a2.collections), [type(c).__name__ for c in ax2_list][:2])
