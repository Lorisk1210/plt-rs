# Feed KeyEvents through key_press_handler with rcParams keymaps edited: grid, scales, fullscreen ignored, home/back on a toolbar-less figure.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backend_bases import KeyEvent, key_press_handler

plt.rcParams["keymap.grid"] = ["g", "ctrl+g"]
plt.rcParams["keymap.yscale"] = ["L"]
plt.rcParams["keymap.xscale"] = ["k"]
fig, ax = plt.subplots(figsize=(4, 3))
ax.plot([1, 10, 100], [1, 10, 100])
fig.canvas.draw()


def state():
    return (ax.xaxis.get_gridlines()[0].get_visible(), ax.yaxis.get_gridlines()[0].get_visible(),
            ax.get_xscale(), ax.get_yscale())


print(state())
for key in ["g", "ctrl+g", "G", "L", "L", "k", "l", "x", "a"]:
    ev = KeyEvent("key_press_event", fig.canvas, key, x=100, y=100)
    key_press_handler(ev, fig.canvas)
    print(key, state())
print({k: plt.rcParams[k] for k in ("keymap.grid", "keymap.grid_minor", "keymap.yscale", "keymap.quit")})
ev = KeyEvent("key_press_event", fig.canvas, "ctrl+alt+shift+x")
print(ev.key, ev.inaxes)
try:
    plt.rcParams["keymap.grid"] = "g"
    print(plt.rcParams["keymap.grid"])
except ValueError as e:
    print("ValueError", e)
fig.savefig("keys.png")
