# For each numeric column of a DataFrame save a histogram with mean/median lines and a KDE-like smooth curve.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt

rng = np.random.default_rng(14)
df = pd.DataFrame({"age": rng.normal(40, 12, 400), "income": rng.lognormal(10, 0.5, 400) / 1000,
                   "score": rng.beta(2, 5, 400) * 100, "visits": rng.poisson(3, 400).astype(float)})
saved = []
for col in df.columns:
    s = df[col]
    fig, ax = plt.subplots(figsize=(5, 3.2))
    counts, edges, _ = ax.hist(s, bins=25, density=True, alpha=0.6)
    centers = (edges[:-1] + edges[1:]) / 2
    ax.plot(centers, np.convolve(counts, [0.25, 0.5, 0.25], mode="same"), color="k")
    ax.axvline(s.mean(), color="r", label=f"mean {s.mean():.1f}")
    ax.axvline(s.median(), color="g", ls="--", label=f"median {s.median():.1f}")
    ax.legend(fontsize=8)
    ax.set_title(col)
    fig.savefig(f"hist_{col}.png", dpi=40)
    saved.append((col, [t.get_text() for t in ax.get_legend().get_texts()]))
    plt.close(fig)
print(saved)
