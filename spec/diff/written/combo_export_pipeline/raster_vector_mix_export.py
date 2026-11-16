# Export the same heavy figure as PNG, rasterized-mesh PDF and SVG; compare file sizes ordering and check headers.
import os
import numpy as np
import matplotlib.pyplot as plt

plt.rcParams["svg.hashsalt"] = "mix"
z = np.random.default_rng(140).random((150, 150))
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3.5))
a.pcolormesh(z, rasterized=True)
b.plot(z[:, 0], lw=0.5)
b.set_title("vector line")
fig.savefig("mix.png", dpi=60)
fig.savefig("mix.pdf", dpi=60, metadata={"CreationDate": None})
fig.savefig("mix.svg", dpi=60, metadata={"Date": None})
for name in ["mix.png", "mix.pdf", "mix.svg"]:
    with open(name, "rb") as f:
        print(name, f.read(6))
print(os.path.getsize("mix.svg") > 1000, os.path.getsize("mix.pdf") > 1000)
