# Travelling wave saved as a short GIF with FuncAnimation and the Pillow writer.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation, PillowWriter

x = np.linspace(0, 2 * np.pi, 120)
fig, ax = plt.subplots(figsize=(4, 3), dpi=60)
line, = ax.plot(x, np.sin(x))
ax.set_ylim(-1.2, 1.2)
frames_seen = []


def update(frame):
    line.set_ydata(np.sin(x - frame / 4))
    frames_seen.append(frame)
    return (line,)


anim = FuncAnimation(fig, update, frames=8, interval=50, blit=True)
anim.save("wave.gif", writer=PillowWriter(fps=10))
print(frames_seen)
from PIL import Image
with Image.open("wave.gif") as im:
    print(im.n_frames, im.size)
