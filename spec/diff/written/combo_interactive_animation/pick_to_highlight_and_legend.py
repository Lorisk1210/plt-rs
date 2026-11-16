# Clicking legend entries toggles the matching lines (pick on legend lines), simulated with pick calls.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.backend_bases import MouseEvent

x = np.linspace(0, 2 * np.pi, 100)
fig, ax = plt.subplots(figsize=(6, 4))
lines = [ax.plot(x, np.sin(x + k), label=f"phase {k}")[0] for k in range(3)]
leg = ax.legend(loc="upper right")
lookup = {}
for leg_line, line in zip(leg.get_lines(), lines):
    leg_line.set_picker(5)
    lookup[leg_line] = line
def on_pick(event):
    line = lookup.get(event.artist)
    if line is not None:
        line.set_visible(not line.get_visible())
        event.artist.set_alpha(1.0 if line.get_visible() else 0.2)
fig.canvas.mpl_connect("pick_event", on_pick)
fig.canvas.draw()
for leg_line in leg.get_lines()[:2]:
    xd, yd = leg_line.get_xydata().mean(axis=0)
    px, py = leg_line.get_transform().transform((xd, yd))
    fig.pick(MouseEvent("button_press_event", fig.canvas, px, py, 1))
print([l.get_visible() for l in lines], [l.get_alpha() for l in leg.get_lines()])
fig.savefig("legend_toggle.png", dpi=50)
