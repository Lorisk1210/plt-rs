# ArtistAnimation from a list of image artists (heat diffusion), saved with writer='pillow'.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.animation import ArtistAnimation

field = np.zeros((30, 30))
field[12:18, 12:18] = 1.0
fig, ax = plt.subplots(figsize=(3, 3), dpi=60)
artists = []
for step in range(6):
    im = ax.imshow(field, cmap="hot", vmin=0, vmax=1, animated=True)
    txt = ax.text(1, 2, f"step {step}", color="w")
    artists.append([im, txt])
    field = field + 0.2 * (np.roll(field, 1, 0) + np.roll(field, -1, 0) + np.roll(field, 1, 1) + np.roll(field, -1, 1) - 4 * field)
ani = ArtistAnimation(fig, artists, interval=100, repeat_delay=500)
ani.save("diffusion.gif", writer="pillow", fps=5)
print(len(artists), round(float(field.sum()), 6))
