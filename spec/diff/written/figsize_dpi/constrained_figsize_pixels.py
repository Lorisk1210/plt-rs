# Produce an image of exactly 800x450 pixels for a web banner, regardless of the default dpi.
import numpy as np
import matplotlib.pyplot as plt
from PIL import Image

w_px, h_px, dpi = 800, 450, 100
fig = plt.figure(figsize=(w_px / dpi, h_px / dpi), dpi=dpi, layout="constrained")
ax = fig.add_subplot()
x = np.linspace(-2, 2, 300)
ax.fill_between(x, np.exp(-x ** 2), color="orchid", alpha=0.6)
ax.set_title("Banner", fontsize=20)
fig.savefig("banner.png", dpi=dpi)
with Image.open("banner.png") as im:
    print(im.size)
ax.set_xlabel("x")
ax.set_ylabel("density")
fig.savefig("banner_2x.png", dpi=2 * dpi)
with Image.open("banner_2x.png") as im:
    print(im.size)
