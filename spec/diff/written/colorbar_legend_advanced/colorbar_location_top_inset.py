# Colour bar on top via location='top', and a colour bar with drawedges for discrete steps.
import numpy as np
import matplotlib.pyplot as plt

z = np.arange(36).reshape(6, 6) % 5
fig, (a, b) = plt.subplots(1, 2, figsize=(9, 4))
im = a.imshow(z, cmap=plt.get_cmap("Set3", 5))
fig.colorbar(im, ax=a, location="top", label="class")
im2 = b.imshow(z, cmap=plt.get_cmap("viridis", 5))
cb = fig.colorbar(im2, ax=b, drawedges=True)
print(cb.orientation, len(cb.dividers.get_segments()))
fig.savefig("top_bar.png")
fig.savefig("colorbar_location_top_inset_small.png", dpi=40)
print(fig.get_size_inches().tolist(), len(fig.axes))
