# Build captions with f-strings from getter values (limits, ticks, colours, positions) and draw them into the figure.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors

fig, ax = plt.subplots(figsize=(7, 4.5))
line, = ax.plot(np.linspace(0, 3, 30), np.exp(np.linspace(0, 3, 30)))
fig.canvas.draw()
x0, x1 = ax.get_xlim()
caption = (f"x from {x0:.2f} to {x1:.2f}; {len(ax.get_xticks())} ticks; colour {mcolors.to_hex(line.get_color())}; "
           f"axes at {tuple(round(v, 2) for v in ax.get_position().bounds)}")
fig.text(0.5, 0.01, caption, ha="center", fontsize=7)
ax.set_title(f"{line.get_label()} with {len(line.get_xdata())} points, max {line.get_ydata().max():.1f}")
print(caption)
print(ax.get_title())
fig.savefig("caption_from_getters.png", dpi=50)
