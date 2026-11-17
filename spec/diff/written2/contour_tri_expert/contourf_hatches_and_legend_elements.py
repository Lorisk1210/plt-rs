# contourf hatches with extend, legend_elements of a ContourSet for lines and fills, and proxy artists' properties.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

x = np.linspace(-2, 2, 40)
X, Y = np.meshgrid(x, x)
Z = np.exp(-(X ** 2 + Y ** 2)) - 0.5 * np.exp(-((X - 1) ** 2 + (Y - 1) ** 2))
fig, (a1, a2) = plt.subplots(1, 2, figsize=(7, 3))
cf = a1.contourf(X, Y, Z, levels=[-0.3, 0, 0.3, 0.6], hatches=["/", "\\", "x", "."], cmap="gray", extend="both", alpha=0.5)
handles, labels = cf.legend_elements(str_format="{:.1f}".format)
print(len(handles), labels, [type(h).__name__ for h in handles][:2], [h.get_hatch() for h in handles])
a1.legend(handles, labels, fontsize=6)
cs = a2.contour(X, Y, Z, levels=[-0.2, 0.2, 0.5], colors=["b", "g", "r"], linestyles=["--", "-", ":"])
h2, l2 = cs.legend_elements()
print(len(h2), l2, [h.get_linestyle() for h in h2], [h.get_color() for h in h2])
a2.legend(h2, l2, loc="lower left")
fig.canvas.draw()
print([t.get_text() for t in a1.get_legend().get_texts()], [t.get_text() for t in a2.get_legend().get_texts()])
print(cf.hatches, cs.linestyles)
with plt.rc_context({"contour.negative_linestyle": "dashdot"}):
    cs2 = a2.contour(X, Y, Z, levels=[-0.2, 0.2], colors="k")
    print(cs2.linestyles)
fig.savefig("contour_legend.png")
cf.hatches = ["o", "", "+", "-", "|"]
fig.canvas.draw()
print(cf.hatches, len(cf.get_paths()))
h3, l3 = cs.legend_elements(variable_name="z")
print(l3[:2], h3[0].get_color())
