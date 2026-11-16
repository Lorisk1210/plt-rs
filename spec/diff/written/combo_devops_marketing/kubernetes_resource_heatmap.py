# Cluster resources: CPU and memory heat maps per node and hour, a limit line plot and a pod count bar.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(202)
nodes = [f"node-{i}" for i in range(8)]
cpu = np.clip(rng.normal(0.55, 0.2, (8, 24)) + 0.2 * np.sin(np.arange(24) / 24 * 2 * np.pi), 0, 1)
mem = np.clip(rng.normal(0.65, 0.1, (8, 24)), 0, 1)
fig, axs = plt.subplots(2, 2, figsize=(12, 7))
for ax, data, title in [(axs[0, 0], cpu, "CPU"), (axs[0, 1], mem, "memory")]:
    im = ax.imshow(data, aspect="auto", cmap="RdYlGn_r", vmin=0, vmax=1)
    ax.set_yticks(range(8), nodes)
    ax.set_title(title)
    fig.colorbar(im, ax=ax, format="{x:.0%}")
axs[1, 0].plot(cpu.mean(axis=0), label="cpu avg")
axs[1, 0].plot(cpu.max(axis=0), label="cpu max")
axs[1, 0].axhline(0.9, color="r", ls="--")
axs[1, 0].legend()
axs[1, 1].bar(nodes, rng.integers(10, 60, 8))
axs[1, 1].tick_params(axis="x", rotation=45)
fig.tight_layout()
print(round(float(cpu.mean()), 4), int((cpu > 0.9).sum()))
fig.savefig("cluster.png", dpi=50)
