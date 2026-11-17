# A tiny plotting API wrapping pyplot with defaults merged from kwargs, returning the axes; repeated calls reuse or create figures.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

DEFAULTS = {"line": {"lw": 2, "color": "C3"}, "bar": {"width": 0.6, "color": "C2"}, "scatter": {"s": 30, "marker": "D"}}


def plot(kind, *args, ax=None, new=False, **kwargs):
    if new:
        plt.figure()
    ax = ax or plt.gca()
    options = {**DEFAULTS.get(kind, {}), **kwargs}
    method = {"line": ax.plot, "bar": ax.bar, "scatter": ax.scatter}[kind]
    method(*args, **options)
    return ax


a = plot("line", [1, 2, 3])
b = plot("line", [3, 2, 1], color="k")
print(a is b, [l.get_color() for l in a.lines], [l.get_linewidth() for l in a.lines])
c = plot("bar", [0, 1], [2, 3], new=True)
print(c is a, [round(p.get_width(), 2) for p in c.patches], len(plt.get_fignums()))
d = plot("scatter", [0, 1], [1, 0], ax=a, marker="o")
print(d is a, len(a.collections), a.collections[0].get_sizes().tolist())
try:
    plot("pie", [1, 2])
except KeyError as e:
    print("KeyError", e)
print(plt.gca() is c, plt.gcf().number, sorted(plt.get_fignums()))
plt.sca(a)
e = plot("line", [0, 0])
print(e is a, len(a.lines))
for num in plt.get_fignums():
    plt.figure(num).savefig(f"api_{num}.png")
plt.close("all")
print(plt.get_fignums())
