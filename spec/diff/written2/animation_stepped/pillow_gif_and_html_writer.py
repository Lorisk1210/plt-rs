# Save one animation as a GIF with PillowWriter and as HTML frames with HTMLWriter; read the GIF back with Pillow.
import os
import numpy as np
import itertools
import uuid
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import animation
from PIL import Image

# The page's element ids come from uuid4: made reproducible, as one does to diff the HTML.
_ids = itertools.count(1)
uuid.uuid4 = lambda: uuid.UUID(int=next(_ids))

fig, ax = plt.subplots(figsize=(2, 2), dpi=40)
ax.set_xlim(0, 1)
ax.set_ylim(0, 1)
dot, = ax.plot([], [], "o", ms=10)


def update(i):
    t = i / 9
    dot.set_data([t], [t ** 2])
    return (dot,)


anim = animation.FuncAnimation(fig, update, frames=10, interval=100)
anim.save("dot.gif", writer=animation.PillowWriter(fps=10))
with Image.open("dot.gif") as im:
    print(im.format, im.size, im.n_frames, im.info.get("duration"), im.info.get("loop"))
html = animation.HTMLWriter(fps=5, embed_frames=False, default_mode="once")
anim.save("dot.html", writer=html)
print(os.path.exists("dot.html"), sorted(os.listdir("dot_frames"))[:3], len(os.listdir("dot_frames")))
with open("dot.html") as f:
    text = f.read()
print("animation" in text.lower(), text.count("dot_frames/frame"), 'once' in text)
print(animation.writers.is_available("pillow"), animation.writers.is_available("html"), "pillow" in animation.writers.list())
print(sorted(animation.PillowWriter.supported_formats if hasattr(animation.PillowWriter, "supported_formats") else []))
print(animation.HTMLWriter.supported_formats)
os.remove("dot.html")
print(sorted(f for f in os.listdir(".") if f.startswith("dot")))
