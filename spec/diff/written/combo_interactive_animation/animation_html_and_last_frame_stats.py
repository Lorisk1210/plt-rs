# Wave animation exported to JS HTML; frame count read from the HTML, and the last frame's line data printed.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib import animation

x = np.linspace(0, 2 * np.pi, 50)
fig, (a, b) = plt.subplots(2, 1, figsize=(4, 4), dpi=40)
l1, = a.plot(x, np.sin(x))
l2, = b.plot(x, np.cos(x), color="C1")
for ax in (a, b):
    ax.set_ylim(-1.1, 1.1)
def step(i):
    l1.set_ydata(np.sin(x - i / 3))
    l2.set_ydata(np.cos(x - i / 2))
    return l1, l2
anim = animation.FuncAnimation(fig, step, frames=6, interval=100)
frames = anim.to_jshtml(fps=5).count("data:image/png;base64")
print(frames, np.round(l1.get_ydata()[:3], 4).tolist(), np.round(l2.get_ydata()[:3], 4).tolist())
fig.savefig("last_frame.png")
