# Animation.save with progress_callback, savefig_kwargs (facecolor) and extra_anim combining two animations into one GIF.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import animation
from PIL import Image

fig, (a1, a2) = plt.subplots(1, 2, figsize=(4, 2), dpi=30)
l1, = a1.plot([0, 1], [0, 0])
l2, = a2.plot([0, 1], [0, 0], "r")
for a in (a1, a2):
    a.set_ylim(-1, 1)
anim1 = animation.FuncAnimation(fig, lambda i: l1.set_ydata([0, np.sin(i)]) or (l1,), frames=6)
anim2 = animation.FuncAnimation(fig, lambda i: l2.set_ydata([0, np.cos(i)]) or (l2,), frames=6)
progress = []
anim1.save("both.gif", writer=animation.PillowWriter(fps=6), extra_anim=[anim2],
           progress_callback=lambda i, n: progress.append((i, n)),
           savefig_kwargs={"facecolor": "lightyellow"})
print(progress)
with Image.open("both.gif") as im:
    print(im.n_frames, im.size)
    im.seek(0)
    rgb = im.convert("RGB")
    print(rgb.getpixel((1, 1)))
print(round(l1.get_ydata()[1], 4), round(l2.get_ydata()[1], 4))
try:
    anim1.save("bad.gif", writer=animation.PillowWriter(fps=6), savefig_kwargs={"bbox_inches": "tight"})
except Exception as e:
    print(type(e).__name__)
progress.clear()
anim2.save("one.gif", writer="pillow", fps=3, dpi=20, progress_callback=lambda i, n: progress.append(i))
with Image.open("one.gif") as im:
    print(im.n_frames, im.size, progress)
