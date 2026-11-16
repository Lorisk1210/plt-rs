# Callbacks on draw_event and xlim_changed to keep a label in sync with the view.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3.5))
ax.plot(np.arange(100), np.sin(np.arange(100) / 8))
status = ax.text(0.02, 0.95, "", transform=ax.transAxes, va="top")
events = []


def on_xlim(axes):
    lo, hi = axes.get_xlim()
    status.set_text(f"view {lo:.0f}..{hi:.0f}")
    events.append("xlim")


ax.callbacks.connect("xlim_changed", on_xlim)
fig.canvas.mpl_connect("draw_event", lambda e: events.append("draw"))
ax.set_xlim(10, 50)
fig.canvas.draw()
ax.set_xlim(20, 30)
fig.savefig("synced.png")
print(events, status.get_text())
