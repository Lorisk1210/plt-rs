# Update an existing scatter's positions, sizes and colours (as in a manual animation step).
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(8)
pos = rng.uniform(0, 1, (40, 2))
fig, ax = plt.subplots(figsize=(4.5, 4.5))
sc = ax.scatter(pos[:, 0], pos[:, 1], c=np.zeros(40), cmap="cool", vmin=0, vmax=1, s=30)
ax.set_xlim(0, 1)
ax.set_ylim(0, 1)
fig.savefig("step0.png")
for step in range(1, 4):
    pos = (pos + rng.normal(0, 0.03, pos.shape)) % 1
    sc.set_offsets(pos)
    sc.set_sizes(np.full(40, 30 + 20 * step))
    sc.set_array(np.full(40, step / 3))
    fig.savefig(f"step{step}.png")
print(sc.get_offsets().shape, sc.get_sizes()[0])
