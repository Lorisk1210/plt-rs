# Interact with an unpickled figure as with the original: events, legend picking, savefig, and new axes added to it.
import pickle
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent, KeyEvent, key_press_handler

fig, ax = plt.subplots(figsize=(4, 3), dpi=80)
ax.plot([0, 1], [0, 1], label="a", picker=5)
ax.legend().set_draggable(True)
f2 = pickle.loads(pickle.dumps(fig))
a2 = f2.axes[0]
f2.canvas.draw()
picked = []
f2.canvas.mpl_connect("pick_event", lambda e: picked.append(type(e.artist).__name__))
x, y = a2.transData.transform((0.5, 0.5))
MouseEvent("button_press_event", f2.canvas, x, y, 1)._process()
print(picked)
key_press_handler(KeyEvent("key_press_event", f2.canvas, "g", x, y), f2.canvas)
print(a2.xaxis.get_gridlines()[0].get_visible())
leg = a2.get_legend()
print(leg.get_draggable(), type(leg._draggable).__name__ if leg._draggable else None)
new = f2.add_subplot(2, 2, 4)
new.plot([1, 2])
f2.canvas.draw()
print(len(f2.axes), [round(v, 3) for v in new.get_position().bounds])
f2.savefig("unpickled.png")
print(f2.canvas.figure is f2, f2.get_dpi(), tuple(f2.get_size_inches()))
print(plt.fignum_exists(f2.number), plt.gcf() is f2)
f2.canvas.mpl_connect("key_press_event", lambda e: picked.append(e.key))
KeyEvent("key_press_event", f2.canvas, "q", x, y)._process()
print(picked[-1], plt.get_fignums())
