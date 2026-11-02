"""30 frames of a travelling wave packet rendered into the page of
matplotlib's JavaScript player (``to_jshtml``)."""

import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation

from _common import save

x = np.linspace(0.0, 10.0, 400)
frames = 30
centres = 2.0 + 6.0 * np.arange(frames) / (frames - 1)
packets = [np.exp(-((x - c) ** 2) / 1.5) * np.cos(4.0 * (x - c)) for c in centres]


def draw():
    fig, ax = plt.subplots(figsize=(6.0, 3.2))
    (wave,) = ax.plot(x, packets[0], linewidth=2.0)
    ax.set_xlim(0.0, 10.0)
    ax.set_ylim(-1.2, 1.2)
    ax.set_title("A travelling wave packet")

    def update(frame):
        wave.set_ydata(packets[frame])
        return (wave,)

    return FuncAnimation(fig, update, frames=frames, interval=80)


if __name__ == "__main__":
    save(draw())
