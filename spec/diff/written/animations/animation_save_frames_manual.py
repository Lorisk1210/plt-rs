# The manual way: update artists in a loop and savefig each frame (no animation module).
import numpy as np
import matplotlib.pyplot as plt

t = np.linspace(0, 1, 100)
fig, ax = plt.subplots(figsize=(4, 3), dpi=60)
line, = ax.plot(t, t * 0)
ax.set_ylim(-1, 1)
title = ax.set_title("")
for k in range(5):
    line.set_ydata(np.sin(2 * np.pi * (t - k / 10)))
    title.set_text(f"frame {k}")
    fig.savefig(f"frame_{k:02d}.png")
print(title.get_text())
plt.close(fig)
