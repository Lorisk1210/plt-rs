# Default artist labels (_child), labels starting with underscore, legend handles picked up, and get_legend_handles_labels order.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots()
a, = ax.plot([0, 1])
b, = ax.plot([0, 1], label="shown")
c, = ax.plot([0, 1], label="_hidden")
d = ax.bar([0], [1], label="bars")
e = ax.scatter([0], [0], label="dots")
f = ax.fill_between([0, 1], 0, 1, label="area")
g = ax.axhline(0.5, label="ref")
print([x.get_label() for x in (a, b, c, e, f, g)], d.get_label(), d.patches[0].get_label())
h, l = ax.get_legend_handles_labels()
print(l, [type(x).__name__ for x in h])
leg = ax.legend()
print([t.get_text() for t in leg.get_texts()])
a.set_label("now shown")
print(ax.get_legend_handles_labels()[1])
leg2 = ax.legend(handles=[a, c], labels=["first", "second"])
print([t.get_text() for t in leg2.get_texts()])
ax.legend([b, e], ["only b", "only e"])
print([t.get_text() for t in ax.get_legend().get_texts()])
import warnings
fig2, ax2 = plt.subplots()
ax2.plot([0, 1])
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    leg3 = ax2.legend()
print([str(w.message)[:60] for w in caught], len(leg3.get_texts()))
print(fig.legends, len(ax.get_children()) > 10)
ax.set_label("the axes")
print(ax.get_label(), fig.get_label())
