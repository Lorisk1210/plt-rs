# ArtistAnimation of prebuilt frames stepped by hand through its private frame sequence, with repeat_delay and blit off.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import animation

fig, ax = plt.subplots(figsize=(3, 3), dpi=50)
frames = []
for k in range(6):
    img = ax.imshow(np.add.outer(np.arange(8), np.arange(8) * k) % 7, animated=True, cmap="viridis")
    txt = ax.text(0.5, 1.02, f"frame {k}", transform=ax.transAxes, ha="center", animated=True)
    frames.append([img, txt])
anim = animation.ArtistAnimation(fig, frames, interval=50, repeat_delay=200, blit=False)
print(len(anim._framedata), [len(f) for f in anim._framedata][:3])
seq = anim.new_frame_seq()
visible = []
for k, artists in zip(range(8), seq):
    anim._draw_frame(artists)
    fig.canvas.draw()
    visible.append([i for i, f in enumerate(frames) if f[0].get_visible()])
print(visible)
print(anim._repeat_delay if hasattr(anim, "_repeat_delay") else None, anim.repeat if hasattr(anim, "repeat") else anim._repeat)
anim.save("artists.gif", writer="pillow", fps=5)
from PIL import Image
with Image.open("artists.gif") as im:
    print(im.n_frames, im.size)
print(all(f[0].get_animated() for f in frames))
print(type(anim.event_source).__name__, anim.event_source.interval)
anim.pause()
anim.resume()
print([f[1].get_text() for f in frames[:2]], frames[0][1].get_visible())
