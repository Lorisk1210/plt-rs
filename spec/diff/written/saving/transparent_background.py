# Logo-like donut chart saved with a transparent background so it can sit on a coloured web page.
import matplotlib.pyplot as plt
from PIL import Image

fig, ax = plt.subplots(figsize=(3, 3))
wedges, _ = ax.pie([45, 30, 25], colors=["#1b9e77", "#d95f02", "#7570b3"],
                   wedgeprops=dict(width=0.35, edgecolor="white"), startangle=90)
ax.text(0, 0, "45%", ha="center", va="center", fontsize=20, fontweight="bold")
ax.set_aspect("equal")

fig.savefig("donut_transparent.png", transparent=True, dpi=100)
fig.savefig("donut_white.png", transparent=False, dpi=100)

with Image.open("donut_transparent.png") as im:
    print(im.mode, im.size)
    print("corner pixel", im.getpixel((0, 0)))
with Image.open("donut_white.png") as im:
    print(im.mode, im.size)
    print("corner pixel", im.getpixel((0, 0)))
