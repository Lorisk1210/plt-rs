# plt.subplots inside a function used twice, returning figures that keep their own state.
import numpy as np
import matplotlib.pyplot as plt


def report(values, title):
    fig, ax = plt.subplots(figsize=(4, 3))
    ax.plot(values)
    ax.set_title(title)
    return fig


f1 = report(np.arange(10), "rising")
f2 = report(np.arange(10)[::-1], "falling")
print(f1.number, f2.number, plt.gcf() is f2)
f1.axes[0].axhline(5, color="r")
f1.savefig("rising.png")
f2.savefig("falling.png")
plt.close(f1)
print(plt.get_fignums())
