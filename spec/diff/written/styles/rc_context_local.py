# Temporarily change fonts and line widths for one figure with rc_context, as a decorator too.
import numpy as np
import matplotlib.pyplot as plt
import matplotlib as mpl

with mpl.rc_context({"lines.linewidth": 4, "font.size": 14, "axes.titleweight": "bold"}):
    fig, ax = plt.subplots(figsize=(5, 3.5))
    ax.plot(np.arange(5), [1, 3, 2, 4, 3])
    ax.set_title("inside rc_context")
    fig.savefig("thick.png")
print(mpl.rcParams["lines.linewidth"])


@mpl.rc_context({"axes.prop_cycle": mpl.cycler(color=["k", "r"]), "lines.linestyle": "--"})
def plot_pair():
    fig, ax = plt.subplots(figsize=(5, 3.5))
    ax.plot([1, 2], [1, 2])
    ax.plot([1, 2], [2, 1])
    fig.savefig("decorated.png")
    return [l.get_color() for l in ax.lines]


print(plot_pair(), mpl.rcParams["lines.linestyle"])
