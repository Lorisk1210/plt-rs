# FuncAnimation with init_func, fargs and a frames generator; frames saved as PNG files by hand.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation

fig, ax = plt.subplots(figsize=(4, 3), dpi=60)
scat = ax.scatter([], [])
ax.set_xlim(-1.2, 1.2)
ax.set_ylim(-1.2, 1.2)
ax.set_aspect("equal")
calls = []


def init():
    scat.set_offsets(np.empty((0, 2)))
    calls.append("init")
    return (scat,)


def update(angle, radius):
    pts = radius * np.c_[np.cos(angle + np.arange(5)), np.sin(angle + np.arange(5))]
    scat.set_offsets(pts)
    calls.append(round(float(angle), 2))
    return (scat,)


ani = FuncAnimation(fig, update, frames=(a / 2 for a in range(4)), init_func=init, fargs=(1.0,), save_count=4)
ani.save("orbit.gif", writer="pillow", fps=4)
print(calls)
