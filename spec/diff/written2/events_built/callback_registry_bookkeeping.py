# Connect and disconnect many handlers through mpl_connect; ids, weak references to bound methods, and exceptions in handlers.
import gc
import weakref
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import KeyEvent

fig, ax = plt.subplots()
log = []


class Listener:
    def __init__(self, name):
        self.name = name

    def on_key(self, event):
        log.append((self.name, event.key))


listeners = [Listener(f"L{k}") for k in range(3)]
cids = [fig.canvas.mpl_connect("key_press_event", l.on_key) for l in listeners]
cids.append(fig.canvas.mpl_connect("key_press_event", lambda e: log.append(("lambda", e.key))))
print(cids == sorted(cids), len(set(cids)))
KeyEvent("key_press_event", fig.canvas, "a")._process()
print(log)
log.clear()
fig.canvas.mpl_disconnect(cids[1])
third = weakref.ref(listeners[2])
del listeners[2]
third_gone = gc.collect() >= 0 and third() is None
KeyEvent("key_press_event", fig.canvas, "b")._process()
print(log)
log.clear()
fig.canvas.mpl_disconnect(cids[1])
fig.canvas.mpl_disconnect(999999)
cb = fig.canvas.callbacks
print(sorted(cb.callbacks), len(cb.callbacks.get("key_press_event", {})))


def boom(event):
    raise RuntimeError("handler failed")


fig.canvas.mpl_connect("key_press_event", boom)
try:
    KeyEvent("key_press_event", fig.canvas, "c")._process()
except RuntimeError as e:
    print("RuntimeError:", e)
print(log)
print(type(cb).__name__, cb.exception_handler.__name__ if hasattr(cb.exception_handler, "__name__") else cb.exception_handler)
