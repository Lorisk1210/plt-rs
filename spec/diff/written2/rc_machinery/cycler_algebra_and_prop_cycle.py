# cycler algebra (+, *, slicing, concat, by_key, change_key) and the resulting prop_cycle on axes, including set_prop_cycle(None).
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from cycler import cycler, concat

c1 = cycler(color=["r", "g", "b"])
c2 = cycler(linestyle=["-", "--", ":"])
c3 = cycler(lw=[1, 2])
summed = c1 + c2
product = c1 * c3
print(len(summed), len(product), summed.keys, sorted(product.keys))
print(list(summed)[:2], list(product)[:3])
print(summed[::2].by_key(), list(concat(c1, cycler(color=["k"]))))
cc = cycler(c1)
cc.change_key("color", "mfc")
print(cc.keys, c1.keys)
fig, ax = plt.subplots()
ax.set_prop_cycle(summed * cycler(marker=["o", "s"]))
lines = [ax.plot([0, 1], [k, k])[0] for k in range(7)]
print([(l.get_color(), l.get_linestyle(), l.get_marker()) for l in lines])
ax.set_prop_cycle(None)
print(ax.plot([0, 1])[0].get_color())
ax.set_prop_cycle("color", ["#111111", "#222222"])
print([ax.plot([0, 1])[0].get_color() for _ in range(3)])
ax.set_prop_cycle(color=["c", "m"], linewidth=[3, 4])
print([(l.get_color(), l.get_linewidth()) for l in (ax.plot([0])[0], ax.plot([0])[0])])
try:
    ax.set_prop_cycle(c1 + cycler(color=["k", "w", "y"]))
except ValueError as e:
    print("ValueError", str(e)[:60])
try:
    ax.set_prop_cycle(cycler(colour=["k"]))
except (ValueError, TypeError) as e:
    print(type(e).__name__, str(e)[:60])
with plt.rc_context({"axes.prop_cycle": cycler(color=["olive", "navy"])}):
    fig2, ax2 = plt.subplots()
    print([ax2.plot([0])[0].get_color() for _ in range(3)])
fig.savefig("cycles.png")
