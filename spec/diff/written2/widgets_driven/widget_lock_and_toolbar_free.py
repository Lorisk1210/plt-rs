# The canvas widgetlock between two selectors on one axes: a drag goes to the one that holds the lock; locks released by hand.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent
from matplotlib.widgets import RectangleSelector, SpanSelector, LockDraw

fig, ax = plt.subplots(figsize=(4, 4), dpi=100)
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
fig.canvas.draw()
log = []
rect = RectangleSelector(ax, lambda a, b: log.append("rect"), useblit=False)
span = SpanSelector(ax, lambda lo, hi: log.append("span"), "horizontal", useblit=False)


def drag(p0, p1):
    x0, y0 = ax.transData.transform(p0)
    x1, y1 = ax.transData.transform(p1)
    MouseEvent("button_press_event", fig.canvas, x0, y0, 1)._process()
    MouseEvent("motion_notify_event", fig.canvas, x1, y1, 1)._process()
    MouseEvent("button_release_event", fig.canvas, x1, y1, 1)._process()


drag((1, 1), (4, 4))
print(log, fig.canvas.widgetlock.locked())
log.clear()
fig.canvas.widgetlock(rect)
drag((1, 1), (5, 5))
print(log, fig.canvas.widgetlock.isowner(rect), fig.canvas.widgetlock.available(span))
fig.canvas.widgetlock.release(rect)
log.clear()
drag((2, 2), (6, 6))
print(log, fig.canvas.widgetlock.locked())
lock = LockDraw()
owner = object()
lock(owner)
try:
    lock(object())
except ValueError as e:
    print("ValueError", e)
print(lock.locked(), lock.isowner(owner))
try:
    lock.release(object())
except ValueError as e:
    print("ValueError", e)
lock.release(owner)
print(lock.locked())
fig.savefig("locks.png")
