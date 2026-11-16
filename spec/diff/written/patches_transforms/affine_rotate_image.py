# Rotate and skew an image inside the axes with an Affine2D added to transData.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib.transforms as mtransforms

z = np.add.outer(np.arange(20), np.arange(20)) % 5
fig, axs = plt.subplots(1, 2, figsize=(8, 4))
for ax, tr in zip(axs, [mtransforms.Affine2D().rotate_deg(30), mtransforms.Affine2D().skew_deg(15, 0)]):
    im = ax.imshow(z, interpolation="none", origin="lower", extent=[0, 20, 0, 20], clip_on=True)
    im.set_transform(tr + ax.transData)
    x1, x2, y1, y2 = im.get_extent()
    ax.plot([x1, x2, x2, x1, x1], [y1, y1, y2, y2, y1], "y--", transform=tr + ax.transData)
    ax.set_xlim(-15, 25)
    ax.set_ylim(-5, 35)
print(np.round(mtransforms.Affine2D().rotate_deg(30).get_matrix(), 4).tolist())
fig.savefig("affine.png")
