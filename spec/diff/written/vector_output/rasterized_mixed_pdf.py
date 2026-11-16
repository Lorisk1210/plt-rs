# Mixed vector/raster PDF: rasterize only the heavy pcolormesh and keep axes and text vector.
import numpy as np
import matplotlib.pyplot as plt

z = np.random.default_rng(1).random((200, 200))
fig, ax = plt.subplots(figsize=(4, 3.5))
mesh = ax.pcolormesh(z, rasterized=True)
ax.set_title("rasterized mesh")
ax.set_rasterization_zorder(None)
fig.savefig("mixed.pdf", dpi=72, metadata={"CreationDate": None})
fig.savefig("mixed.svg", dpi=72, metadata={"Date": None})
with open("mixed.svg") as f:
    print(f.read().count("<image"))
print(mesh.get_rasterized())
