# Compare the same plot saved at 72, 150 and 300 dpi to see that text scales with dpi.
import matplotlib.pyplot as plt
from PIL import Image

fig, ax = plt.subplots(figsize=(3, 2))
ax.plot([0, 1, 2], [2, 0, 1], lw=3)
ax.set_title("Scaling check", fontsize=14)
ax.text(1, 1, "label", fontsize=12)
for dpi in (72, 150, 300):
    name = f"scale_{dpi}.png"
    fig.savefig(name, dpi=dpi)
    with Image.open(name) as im:
        print(dpi, im.size)
fig.savefig("scale_figure_dpi.png", dpi="figure")
with Image.open("scale_figure_dpi.png") as im:
    print("figure dpi", fig.dpi, im.size)
