# draw_event, resize_event and close_event handlers on an Agg canvas; what each event carries and how often it fires.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import ResizeEvent, CloseEvent
plt.rcParams["svg.hashsalt"] = "written2"

fig, ax = plt.subplots(figsize=(4, 3), dpi=100)
seen = []
fig.canvas.mpl_connect("draw_event", lambda e: seen.append(("draw", type(e.renderer).__name__)))
fig.canvas.mpl_connect("resize_event", lambda e: seen.append(("resize", e.width, e.height)))
fig.canvas.mpl_connect("close_event", lambda e: seen.append(("close", e.name)))
fig.canvas.draw()
fig.canvas.draw()
fig.savefig("first.png")
print(seen)
seen.clear()
fig.set_size_inches(5, 2)
ResizeEvent("resize_event", fig.canvas)._process()
fig.canvas.draw()
print(seen)
seen.clear()
fig.savefig("vector.svg", metadata={"Date": None})
fig.savefig("vector.pdf", metadata={"CreationDate": None})
print(seen)
seen.clear()
fig.canvas.draw_idle()
print(seen)
CloseEvent("close_event", fig.canvas)._process()
plt.close(fig)
print(seen, plt.get_fignums())
fig2 = plt.figure()
fig2.canvas.mpl_connect("close_event", lambda e: seen.append(("close2", e.canvas.figure is fig2)))
plt.close(fig2)
print(seen[-1:], plt.get_fignums())
