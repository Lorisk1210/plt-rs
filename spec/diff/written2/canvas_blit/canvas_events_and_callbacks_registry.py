# Inspect a canvas' callback registry without pyplot: connect, list, disconnect, process custom signals and count calls.
from matplotlib.figure import Figure
from matplotlib.backends.backend_agg import FigureCanvasAgg
from matplotlib.backend_bases import DrawEvent, ResizeEvent

fig = Figure()
canvas = FigureCanvasAgg(fig)
calls = []
cid_draw = canvas.mpl_connect("draw_event", lambda e: calls.append(("draw", type(e).__name__, e.renderer is not None)))
cid_resize = canvas.mpl_connect("resize_event", lambda e: calls.append(("resize", e.width, e.height)))
print(sorted(canvas.callbacks.callbacks.keys()))
canvas.draw()
canvas.callbacks.process("resize_event", ResizeEvent("resize_event", canvas))
canvas.mpl_disconnect(cid_draw)
canvas.draw()
print(calls)
print(cid_draw != cid_resize, len(canvas.callbacks.callbacks.get("draw_event", {})))
try:
    canvas.mpl_connect("no_such_event", print)
except ValueError as err:
    print("ValueError:", err)
ev = DrawEvent("draw_event", canvas, canvas.get_renderer())
print(ev.name, ev.canvas is canvas)
fig.savefig("callbacks.png")
counter = {"n": 0}
def bump(event):
    counter["n"] += 1
cids = [canvas.mpl_connect("draw_event", bump) for _ in range(3)]
canvas.draw()
print(counter["n"], len(set(cids)))
for c in cids:
    canvas.mpl_disconnect(c)
canvas.draw()
print(counter["n"])
