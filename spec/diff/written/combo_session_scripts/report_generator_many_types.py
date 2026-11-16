# A tiny report generator: a list of chart specs (dicts) rendered by one function into a grid, each with its own type.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(242)
specs = [{"kind": "line", "y": rng.normal(size=30).cumsum(), "title": "walk"},
         {"kind": "bar", "x": list("abcd"), "y": [3, 5, 2, 4], "title": "counts"},
         {"kind": "hist", "y": rng.normal(size=300), "title": "dist"},
         {"kind": "scatter", "x": rng.random(50), "y": rng.random(50), "title": "cloud"},
         {"kind": "pie", "y": [1, 2, 3], "title": "share"},
         {"kind": "image", "y": rng.random((10, 10)), "title": "grid"}]
def render(ax, s):
    k = s["kind"]
    if k == "line": ax.plot(s["y"])
    elif k == "bar": ax.bar(s["x"], s["y"])
    elif k == "hist": ax.hist(s["y"], bins=20)
    elif k == "scatter": ax.scatter(s["x"], s["y"], s=8)
    elif k == "pie": ax.pie(s["y"])
    elif k == "image": ax.figure.colorbar(ax.imshow(s["y"]), ax=ax)
    ax.set_title(s["title"])
fig, axs = plt.subplots(2, 3, figsize=(12, 7))
for ax, s in zip(axs.flat, specs):
    render(ax, s)
fig.tight_layout()
print([ax.get_title() for ax in axs.flat], len(fig.axes))
fig.savefig("generated_report.png", dpi=50)
