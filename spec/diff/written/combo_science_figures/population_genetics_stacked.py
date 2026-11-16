# Admixture-style stacked bars of ancestry proportions sorted by group, with group separators and labels below.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(33)
groups = {"pop1": (0.7, 0.2, 0.1), "pop2": (0.1, 0.8, 0.1), "pop3": (0.3, 0.3, 0.4)}
props, labels = [], []
for g, alpha in groups.items():
    p = rng.dirichlet(np.array(alpha) * 20, 25)
    p = p[np.argsort(-p[:, 0])]
    props.append(p)
    labels += [g] * 25
props = np.vstack(props)
fig, ax = plt.subplots(figsize=(10, 3))
bottom = np.zeros(len(props))
for k, color in enumerate(["#1b9e77", "#d95f02", "#7570b3"]):
    ax.bar(np.arange(len(props)), props[:, k], bottom=bottom, width=1, color=color, label=f"K{k + 1}")
    bottom += props[:, k]
for edge in (25, 50):
    ax.axvline(edge - 0.5, color="k", lw=1.5)
ax.set_xticks([12, 37, 62], list(groups))
ax.set_xlim(-0.5, len(props) - 0.5)
ax.set_ylim(0, 1)
ax.legend(ncols=3, loc="upper center", bbox_to_anchor=(0.5, 1.2))
fig.tight_layout()
print(np.round(props.mean(axis=0), 4).tolist())
fig.savefig("admixture.png", dpi=50)
