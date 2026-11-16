# A Next button cycling the image colormap and a CheckButtons toggling contours; clicked by code, state printed.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.widgets import Button, CheckButtons

y, x = np.mgrid[-2:2:60j, -2:2:60j]
z = np.exp(-(x ** 2 + y ** 2)) * np.cos(3 * x)
fig, ax = plt.subplots(figsize=(6, 5))
fig.subplots_adjust(right=0.75)
im = ax.imshow(z, cmap="viridis")
cs = ax.contour(z, colors="w", linewidths=0.6)
cmaps = ["viridis", "magma", "coolwarm", "gray"]
state = {"k": 0}
def next_cmap(_):
    state["k"] = (state["k"] + 1) % len(cmaps)
    im.set_cmap(cmaps[state["k"]])
btn = Button(fig.add_axes([0.8, 0.8, 0.15, 0.07]), "next")
btn.on_clicked(next_cmap)
chk = CheckButtons(fig.add_axes([0.8, 0.6, 0.15, 0.1]), ["contours"], [True])
chk.on_clicked(lambda label: cs.set_visible(not cs.get_visible()))
for _ in range(3):
    next_cmap(None)
chk.set_active(0)
print(im.get_cmap().name, cs.get_visible(), chk.get_status())
fig.savefig("cmap_button.png", dpi=50)
