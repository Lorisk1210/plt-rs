# Animated particle with a fading trail (LineCollection alphas), a frame counter and a final summary frame saved.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation
from matplotlib.collections import LineCollection

t = np.linspace(0, 4 * np.pi, 80)
path = np.c_[np.cos(t) * t / 10, np.sin(t) * t / 10]
fig, ax = plt.subplots(figsize=(4, 4), dpi=50)
ax.set_xlim(-1.4, 1.4); ax.set_ylim(-1.4, 1.4); ax.set_aspect("equal")
trail = LineCollection([], colors="C0")
ax.add_collection(trail)
dot, = ax.plot([], [], "ro")
counter = ax.text(0.02, 0.95, "", transform=ax.transAxes)
def update(i):
    seg = np.stack([path[max(0, i - 15):i], path[max(1, i - 14):i + 1]], axis=1) if i > 1 else np.empty((0, 2, 2))
    trail.set_segments(seg)
    trail.set_alpha(None)
    trail.set_color([(0.1, 0.4, 0.8, a) for a in np.linspace(0.1, 1, len(seg))])
    dot.set_data([path[i, 0]], [path[i, 1]])
    counter.set_text(f"frame {i}")
    return trail, dot, counter
ani = FuncAnimation(fig, update, frames=range(0, 80, 10), blit=False)
ani.save("trail.gif", writer="pillow", fps=4)
update(79)
fig.savefig("final_frame.png")
print(counter.get_text(), len(trail.get_segments()))
