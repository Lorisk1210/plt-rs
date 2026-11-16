# Pickling a figure whose canvas has lambda callbacks connected: works (callbacks dropped) or not?
import pickle
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3))
ax.plot([1, 2])
fig.canvas.mpl_connect("button_press_event", lambda e: None)
ax.callbacks.connect("xlim_changed", lambda a: None)
try:
    out = pickle.loads(pickle.dumps(fig))
    print("pickled", len(out.axes))
except Exception as err:
    print(type(err).__name__, str(err)[:80])
fig.savefig("callbacks.png")
