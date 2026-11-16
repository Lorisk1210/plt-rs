# Animation exported to JavaScript HTML (to_jshtml) for a notebook, checking the output text.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib import animation

fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
pt, = ax.plot([], [], "o")
ax.set_xlim(0, 1)
ax.set_ylim(0, 1)


def step(i):
    pt.set_data([i / 4], [(i / 4) ** 2])
    return (pt,)


anim = animation.FuncAnimation(fig, step, frames=5, interval=200)
frames = anim.to_jshtml(fps=5, default_mode="once").count("data:image/png;base64")
print(frames)
print(len(anim._save_seq) if hasattr(anim, "_save_seq") else "no seq")
plt.close(fig)
