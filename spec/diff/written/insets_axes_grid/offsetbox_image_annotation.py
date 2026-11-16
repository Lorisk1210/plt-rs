# Put small images (thumbnails) at data points with OffsetImage and AnnotationBbox.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.offsetbox import OffsetImage, AnnotationBbox, TextArea

fig, ax = plt.subplots(figsize=(6, 4))
ax.plot([0, 10], [0, 10], alpha=0)
for k, (x, y) in enumerate([(2, 3), (5, 7), (8, 4)]):
    thumb = np.outer(np.sin(np.linspace(0, k + 1, 20)), np.cos(np.linspace(0, 3, 20)))
    ab = AnnotationBbox(OffsetImage(thumb, zoom=1.5, cmap="plasma"), (x, y), frameon=True, pad=0.2)
    ax.add_artist(ab)
ax.add_artist(AnnotationBbox(TextArea("a text box"), (5, 1), xybox=(30, -20), boxcoords="offset points",
                             arrowprops=dict(arrowstyle="->")))
ax.set_xlim(0, 10)
ax.set_ylim(0, 10)
print(len(ax.artists))
fig.savefig("thumbnails.png")
