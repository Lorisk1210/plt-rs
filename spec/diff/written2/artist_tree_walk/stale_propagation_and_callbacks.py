# The stale flag propagating from artist to axes to figure, stale_callback chains, pchanged callbacks and their removal.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots()
ln, = ax.plot([0, 1])
fig.canvas.draw()
print(ln.stale, ax.stale, fig.stale)
ln.set_color("r")
print(ln.stale, ax.stale, fig.stale)
fig.canvas.draw()
seen = []
cid = ln.add_callback(lambda a: seen.append(("pchanged", type(a).__name__)))
ln.set_linewidth(4)
ln.set_label("x")
ln.set_data([0, 1], [1, 0])
print(seen)
ln.remove_callback(cid)
ln.set_alpha(0.5)
print(len(seen))
fig.canvas.draw()
calls = []
fig.stale_callback = lambda f, val: calls.append(("fig", val))
ln.set_marker("o")
print(calls, fig.stale)
fig.canvas.draw()
print(ln.stale, ax.stale, fig.stale)
ln.stale = True
print(ax.stale)
ax2 = fig.add_axes([0.6, 0.6, 0.2, 0.2])
fig.canvas.draw()
ax2.set_title("x")
print(ax2.stale, ax.stale, fig.stale)
t = fig.text(0.1, 0.1, "free")
fig.canvas.draw()
t.set_text("changed")
print(t.stale, fig.stale)
