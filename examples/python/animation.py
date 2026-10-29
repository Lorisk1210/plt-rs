"""Animation saved as a GIF: a wave packet travelling along a string, its
envelope drawn behind it."""

import numpy as np

import pltrs.pyplot as plt
from pltrs.animation import FuncAnimation

x = np.linspace(0, 10, 400)
frames = 30


def packet(centre):
    """The displacement of a packet centred at `centre`."""
    return np.exp(-((x - centre) ** 2) / 1.5) * np.cos(4 * (x - centre))


def envelope(centre):
    """The envelope of the packet centred at `centre`."""
    return np.exp(-((x - centre) ** 2) / 1.5)


fig, ax = plt.subplots(figsize=(6, 3.2), layout="constrained")
(upper,) = ax.plot(x, envelope(2), color="0.75", linewidth=1, linestyle="--")
(lower,) = ax.plot(x, -envelope(2), color="0.75", linewidth=1, linestyle="--")
(wave,) = ax.plot(x, packet(2), linewidth=2)
ax.set_xlim(0, 10)
ax.set_ylim(-1.2, 1.2)
ax.set_title("A travelling wave packet")
ax.set_xlabel("position")


def update(frame):
    centre = 2 + 6 * frame / (frames - 1)
    upper.set_ydata(envelope(centre))
    lower.set_ydata(-envelope(centre))
    wave.set_ydata(packet(centre))
    return upper, lower, wave


animation = FuncAnimation(fig, update, frames=frames, interval=80)
animation.save("animation.gif", writer="pillow")
