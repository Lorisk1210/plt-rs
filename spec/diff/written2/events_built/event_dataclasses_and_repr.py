# The event classes themselves: attributes, str(), defaults and guiEvent; Event subclasses with custom fields sent through callbacks.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import backend_bases as bb

fig, ax = plt.subplots(figsize=(3, 3), dpi=100)
fig.canvas.draw()
events = [
    bb.Event("custom_event", fig.canvas, guiEvent="gui"),
    bb.LocationEvent("loc", fig.canvas, 150, 150),
    bb.MouseEvent("button_press_event", fig.canvas, 150, 150, button=2, key="alt"),
    bb.KeyEvent("key_press_event", fig.canvas, "enter", 20, 20),
    bb.ResizeEvent("resize_event", fig.canvas),
    bb.CloseEvent("close_event", fig.canvas),
    bb.DrawEvent("draw_event", fig.canvas, fig.canvas.get_renderer()),
]
for ev in events:
    attrs = {k: v for k, v in vars(ev).items() if k not in ("canvas", "renderer", "inaxes", "_guiEvent_deleted")}
    print(type(ev).__name__, ev.name, sorted(attrs))
print(type(events[2]).__str__ is object.__str__, events[2].name, events[2].button, events[2].key)
print(events[3].name, events[3].key, events[3].x, events[3].y)
print(events[0].guiEvent, events[2].guiEvent)
print(events[4].width, events[4].height)


class TickEvent(bb.Event):
    def __init__(self, name, canvas, count):
        super().__init__(name, canvas)
        self.count = count


got = []
try:
    fig.canvas.callbacks.connect("tick_event", lambda e: got.append(e.count))
except ValueError as e:
    print("ValueError:", str(e)[:50])
from matplotlib.cbook import CallbackRegistry
ticks = CallbackRegistry()
ticks.connect("tick_event", lambda e: got.append(e.count))
for k in range(3):
    ticks.process("tick_event", TickEvent("tick_event", fig.canvas, k))
print(got)
pe = bb.PickEvent("pick_event", fig.canvas, events[2], ax.patch, extra="value")
print(pe.artist is ax.patch, pe.extra, pe.mouseevent.button)
print(bb.MouseButton(1), bb.MouseButton.LEFT.name)
