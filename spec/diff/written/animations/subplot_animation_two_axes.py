# Animate two subplots at once (signal and its running histogram) and save a GIF.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import FuncAnimation

rng = np.random.default_rng(4)
samples = rng.normal(0, 1, 200)
fig, (a, b) = plt.subplots(1, 2, figsize=(6, 2.5), dpi=50)
line, = a.plot([], [])
a.set_xlim(0, 200)
a.set_ylim(-3.5, 3.5)
b.set_xlim(-3.5, 3.5)
b.set_ylim(0, 40)


def update(k):
    n = (k + 1) * 40
    line.set_data(np.arange(n), samples[:n])
    for p in list(b.patches):
        p.remove()
    b.hist(samples[:n], bins=15, range=(-3.5, 3.5), color="C1")
    return [line, *b.patches]


ani = FuncAnimation(fig, update, frames=5)
ani.save("running_hist.gif", writer="pillow", fps=2)
print(len(b.patches))
