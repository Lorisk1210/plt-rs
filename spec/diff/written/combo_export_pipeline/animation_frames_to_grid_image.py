# Grab animation frames manually (draw + buffer_rgba), assemble a contact sheet, and save it and a GIF of the frames.
import numpy as np
import matplotlib.pyplot as plt
from PIL import Image

fig, ax = plt.subplots(figsize=(2, 2), dpi=40)
line, = ax.plot([], [])
ax.set_xlim(0, 2 * np.pi); ax.set_ylim(-1, 1); ax.axis("off")
x = np.linspace(0, 2 * np.pi, 60)
frames = []
for k in range(6):
    line.set_data(x, np.sin(x + k / 2))
    fig.canvas.draw()
    frames.append(np.asarray(fig.canvas.buffer_rgba())[..., :3].copy())
sheet = np.concatenate([np.concatenate(frames[:3], axis=1), np.concatenate(frames[3:], axis=1)], axis=0)
plt.imsave("sheet.png", sheet)
Image.fromarray(frames[0]).save("frames.gif", save_all=True, append_images=[Image.fromarray(f) for f in frames[1:]], duration=100)
print(sheet.shape, len(frames), int(np.abs(frames[0].astype(int) - frames[1]).sum() > 0))
