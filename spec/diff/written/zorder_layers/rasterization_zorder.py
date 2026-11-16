# set_rasterization_zorder: everything below z=0 is rasterized in vector output.
import numpy as np
import matplotlib.pyplot as plt

plt.rcParams["svg.hashsalt"] = "layers"
rng = np.random.default_rng(1)
fig, ax = plt.subplots(figsize=(4, 3))
ax.scatter(rng.random(3000), rng.random(3000), s=2, zorder=-1)
ax.plot([0, 1], [0, 1], "r", zorder=2)
ax.set_rasterization_zorder(0)
fig.savefig("rz.svg", dpi=50, metadata={"Date": None})
with open("rz.svg") as f:
    s = f.read()
print(s.count("<image"), ax.get_rasterization_zorder())
fig.savefig("rz.png", dpi=60)
