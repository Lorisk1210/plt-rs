# Save an animation as an HTML file with HTMLWriter and embedded frames.
import os
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation, HTMLWriter

fig, ax = plt.subplots(figsize=(3, 2), dpi=50)
bars = ax.bar(range(4), [1, 2, 3, 4])
ax.set_ylim(0, 6)


def grow(i):
    for b, h in zip(bars, np.array([1, 2, 3, 4]) + i * 0.5):
        b.set_height(h)
    return bars


ani = FuncAnimation(fig, grow, frames=4)
ani.save("bars.html", writer=HTMLWriter(fps=4, embed_frames=True))
with open("bars.html") as f:
    print(f.read().count("data:image/png;base64"), os.path.exists("bars.html"))
