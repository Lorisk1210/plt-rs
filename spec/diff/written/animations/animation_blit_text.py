# Blitted animation with a text artist and a line, rendered frame by frame via to_html5_video fallback check.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation, writers

fig, ax = plt.subplots(figsize=(4, 3), dpi=60)
ln, = ax.plot([], [], "b-")
counter = ax.text(0.05, 0.9, "", transform=ax.transAxes)
ax.set_xlim(0, 2 * np.pi)
ax.set_ylim(-1, 1)
xs = np.linspace(0, 2 * np.pi, 60)


def animate(i):
    ln.set_data(xs[: i * 10 + 2], np.sin(xs[: i * 10 + 2]))
    counter.set_text(f"n={i * 10 + 2}")
    return ln, counter


ani = FuncAnimation(fig, animate, frames=6, blit=True)
print(writers.is_available("pillow"), writers.is_available("ffmpeg_file") in (True, False))
ani.save("growing.gif", writer="pillow", fps=6)
print(counter.get_text())
