# Keyboard shortcuts: a key_press handler toggling grid and line visibility, fed synthetic KeyEvents.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.backend_bases import KeyEvent

fig, ax = plt.subplots(figsize=(5, 3.5))
line, = ax.plot(np.arange(10), np.arange(10) ** 2)
log = []


def on_key(event):
    log.append(event.key)
    if event.key == "v":
        line.set_visible(not line.get_visible())
    elif event.key == "up":
        line.set_linewidth(line.get_linewidth() + 1)


fig.canvas.mpl_connect("key_press_event", on_key)
for key in ["up", "up", "v", "shift+a"]:
    KeyEvent("key_press_event", fig.canvas, key, x=100, y=100)._process()
print(log, line.get_visible(), line.get_linewidth())
fig.savefig("keys.png")
