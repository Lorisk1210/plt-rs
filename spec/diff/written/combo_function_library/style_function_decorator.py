# A decorator applying a house style (rc_context) to plotting functions, used for three different charts.
import functools
import numpy as np
import matplotlib as mpl
import matplotlib.pyplot as plt

HOUSE = {"axes.spines.top": False, "axes.spines.right": False, "axes.grid": True, "grid.alpha": 0.3,
         "axes.prop_cycle": mpl.cycler(color=["#264653", "#2a9d8f", "#e9c46a", "#e76f51"]), "font.size": 9}


def house_style(func):
    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        with mpl.rc_context(HOUSE):
            return func(*args, **kwargs)
    return wrapper


@house_style
def trend(values, title):
    fig, ax = plt.subplots(figsize=(5, 3))
    for k, v in enumerate(values):
        ax.plot(v, label=f"series {k}")
    ax.set_title(title)
    ax.legend()
    return fig, ax


@house_style
def composition(parts):
    fig, ax = plt.subplots(figsize=(4, 4))
    ax.pie(parts.values(), labels=parts.keys())
    return fig, ax


rng = np.random.default_rng(18)
f1, a1 = trend([rng.normal(0, 1, 30).cumsum() for _ in range(3)], "random walks")
f2, a2 = composition({"a": 2, "b": 3, "c": 5})
f1.savefig("trend.png", dpi=50)
f2.savefig("composition.png", dpi=50)
print(trend.__name__, [l.get_color() for l in a1.lines], a1.spines["top"].get_visible())
print(mpl.rcParams["axes.spines.top"], [w.get_facecolor()[:3] for w in a2.patches][:1])
