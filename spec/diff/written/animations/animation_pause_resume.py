# Users call pause()/resume() and new_frame_seq(); the animation still saves all frames.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation

fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
dot, = ax.plot([0], [0], "ro")
ax.set_xlim(0, 5)
ax.set_ylim(0, 5)
seen = []


def move(i):
    dot.set_data([i], [i])
    seen.append(i)
    return (dot,)


ani = FuncAnimation(fig, move, frames=range(5), repeat=False)
ani.pause()
ani.resume()
ani.save("diag.gif", writer="pillow", fps=5)
print(seen)
