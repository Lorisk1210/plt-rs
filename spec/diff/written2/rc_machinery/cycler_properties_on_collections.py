# The property cycle across plot, scatter, bar, fill_between and stackplot: which calls advance it and which colours they take.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from cycler import cycler

plt.rcParams["axes.prop_cycle"] = cycler(color=["#e41a1c", "#377eb8", "#4daf4a", "#984ea3", "#ff7f00", "#a65628"])
fig, ax = plt.subplots(figsize=(5, 4))
x = np.arange(5)
seq = []
seq.append(("plot", ax.plot(x, x)[0].get_color()))
seq.append(("scatter", tuple(np.round(ax.scatter(x, x + 1).get_facecolor()[0], 3))))
seq.append(("bar", tuple(np.round(ax.bar(x, 1, bottom=3).patches[0].get_facecolor(), 3))))
seq.append(("fill_between", tuple(np.round(ax.fill_between(x, 5, 6).get_facecolor()[0], 3))))
polys = ax.stackplot(x, [x * 0 + 1, x * 0 + 2], baseline="zero")
seq.append(("stackplot", [tuple(np.round(p.get_facecolor()[0], 3)) for p in polys]))
seq.append(("errorbar", ax.errorbar(x, x + 7, yerr=0.2).lines[0].get_color()))
seq.append(("step", ax.step(x, x + 8)[0].get_color()))
seq.append(("hist", tuple(np.round(ax.hist([1, 2, 2, 3], bins=3)[2][0].get_facecolor(), 3))))
seq.append(("axhline", ax.axhline(2).get_color()))
seq.append(("text", ax.text(0, 0, "t").get_color()))
seq.append(("plot again", ax.plot(x, -x)[0].get_color()))
for entry in seq:
    print(entry)
print(ax._get_lines.get_next_color(), ax._get_patches_for_fill.get_next_color())
plt.rcdefaults()
fig.savefig("cycle_usage.png")
fig2, ax2 = plt.subplots()
print([ax2.plot([0])[0].get_color() for _ in range(2)], tuple(np.round(ax2.bar([0], [1])[0].get_facecolor(), 3)))
