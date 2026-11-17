# What survives pickling: canvas callbacks (dropped), artist callbacks, picker functions, widgets referencing axes.
import pickle
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.widgets import Slider


def on_pick(event):
    print("picked")


def module_picker(artist, mouseevent):
    return False, {}


fig, ax = plt.subplots()
ln, = ax.plot([0, 1], picker=module_picker)
fig.canvas.mpl_connect("pick_event", on_pick)
fig.canvas.mpl_connect("button_press_event", lambda e: None)
ln.add_callback(print)
sax = fig.add_axes([0.2, 0.02, 0.6, 0.03])
slider = Slider(sax, "s", 0, 1)
fig.slider = None
print(sorted(fig.canvas.callbacks.callbacks))
for what, obj in [("figure", fig), ("line", ln), ("slider", slider)]:
    try:
        copy = pickle.loads(pickle.dumps(obj))
        print(what, "ok", type(copy).__name__)
    except Exception as e:
        print(what, type(e).__name__, str(e)[:60])
f2 = pickle.loads(pickle.dumps(fig))
print(sorted(f2.canvas.callbacks.callbacks), len(f2.canvas.callbacks.callbacks.get("pick_event", {})))
l2 = f2.axes[0].lines[0]
print(l2.get_picker().__name__, len(l2._callbacks.callbacks))
print(len(f2.axes), f2.axes[1].get_label() if len(f2.axes) > 1 else None)
