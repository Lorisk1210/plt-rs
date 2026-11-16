# A generic facet(df, by, kind) function used for line, bar and hist facets of the same DataFrame.
import numpy as np
import pandas as pd
import matplotlib.pyplot as plt


def facet(df, by, value, kind="line", ncols=3, **kw):
    groups = list(df.groupby(by))
    nrows = int(np.ceil(len(groups) / ncols))
    fig, axs = plt.subplots(nrows, ncols, figsize=(3.2 * ncols, 2.6 * nrows), squeeze=False, sharey=kind != "hist")
    for ax, (name, g) in zip(axs.flat, groups):
        if kind == "line":
            ax.plot(g["t"], g[value], **kw)
        elif kind == "bar":
            ax.bar(g["t"], g[value], **kw)
        else:
            ax.hist(g[value], **kw)
        ax.set_title(str(name))
    for ax in axs.flat[len(groups):]:
        ax.set_visible(False)
    fig.tight_layout()
    return fig, axs


rng = np.random.default_rng(20)
df = pd.DataFrame({"g": np.repeat(list("ABCDE"), 12), "t": np.tile(np.arange(12), 5), "v": rng.normal(10, 3, 60)})
f1, a1 = facet(df, "g", "v")
f2, a2 = facet(df, "g", "v", kind="bar", color="C2")
f3, a3 = facet(df, "g", "v", kind="hist", bins=6, ncols=5)
for i, f in enumerate((f1, f2, f3)):
    f.savefig(f"facet_{i}.png", dpi=40)
print(a1.shape, a3.shape, sum(ax.get_visible() for ax in a1.flat), a2[0, 0].get_ylim() == a2[1, 1].get_ylim())
