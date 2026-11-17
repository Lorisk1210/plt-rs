# Cursor and MultiCursor follow motion events across axes; line positions and visibility read back after each move.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent
from matplotlib.widgets import Cursor, MultiCursor

fig, (a1, a2, a3) = plt.subplots(3, 1, figsize=(4, 6), dpi=100, sharex=True)
for k, a in enumerate((a1, a2, a3)):
    a.plot([0, 10], [0, 10 * (k + 1)])
fig.canvas.draw()
cur = Cursor(a1, useblit=False, color="r", linewidth=1, horizOn=True, vertOn=True)
multi = MultiCursor(fig.canvas, (a2, a3), useblit=False, horizOn=True, vertOn=True, color="g")
for ax, xy in [(a1, (2, 3)), (a2, (5, 10)), (a3, (7, 21)), (a1, (9, 9))]:
    x, y = ax.transData.transform(xy)
    MouseEvent("motion_notify_event", fig.canvas, x, y)._process()
    print("cursor", cur.lineh.get_visible(), [round(v, 2) for v in cur.linev.get_xdata()],
          [round(v, 2) for v in cur.lineh.get_ydata()])
    print("multi", [l.get_visible() for l in multi.vlines], [round(l.get_xdata()[0], 2) for l in multi.vlines],
          [round(l.get_ydata()[0], 2) for l in multi.hlines])
MouseEvent("motion_notify_event", fig.canvas, 2, 2)._process()
print(cur.lineh.get_visible(), [l.get_visible() for l in multi.vlines])
cur.visible = False
x, y = a1.transData.transform((4, 4))
MouseEvent("motion_notify_event", fig.canvas, x, y)._process()
print(cur.linev.get_visible(), cur.active)
multi.disconnect()
x, y = a2.transData.transform((1, 1))
MouseEvent("motion_notify_event", fig.canvas, x, y)._process()
print([round(l.get_xdata()[0], 2) for l in multi.vlines])
fig.savefig("cursors.png")
