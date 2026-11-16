# savefig keyword soup copied from answers: format, dpi, bbox, pad, facecolor, transparent, backend.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3))
ax.plot([3, 1, 2])
ax.set_title("kwargs")
fig.savefig("soup.png", format="png", dpi=120, bbox_inches="tight", pad_inches=0.2, facecolor="w",
            edgecolor="none", transparent=False, backend="agg", orientation="portrait")
fig.savefig("soup.pdf", backend="pdf", metadata={"CreationDate": None})
from PIL import Image
with Image.open("soup.png") as im:
    print(im.size, im.mode)
fig.savefig("savefig_kwargs_orientation_check.png", dpi=40)
print(fig.get_size_inches())
