# Blitting pieces: copy_from_bbox of the background, draw an artist, restore_region.
import numpy as np
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3), dpi=60)
ax.plot(np.arange(10))
fig.canvas.draw()
background = fig.canvas.copy_from_bbox(ax.bbox)
dot, = ax.plot([5], [5], "ro", ms=20, animated=True)
ax.draw_artist(dot)
with_dot = np.asarray(fig.canvas.buffer_rgba()).copy()
fig.canvas.restore_region(background)
restored = np.asarray(fig.canvas.buffer_rgba()).copy()
print(int((with_dot != restored).any(axis=2).sum()) > 0)
fig.canvas.blit(ax.bbox)
plt.imsave("with_dot.png", with_dot)
plt.imsave("restored.png", restored)
