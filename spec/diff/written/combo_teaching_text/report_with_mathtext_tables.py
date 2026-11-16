# A results page: mathtext equations in a text panel, a table with math headers, and a plot with a LaTeX-like legend.
import numpy as np
import matplotlib.pyplot as plt

fig = plt.figure(figsize=(10, 6))
t_ax = fig.add_axes([0.05, 0.6, 0.4, 0.35]); t_ax.axis("off")
eqs = [r"$E = \frac{1}{2} m v^2$", r"$\sigma = \sqrt{\frac{1}{N}\sum_i (x_i - \bar{x})^2}$", r"$\hat{y} = \beta_0 + \beta_1 x$"]
for k, e in enumerate(eqs):
    t_ax.text(0, 0.8 - k * 0.35, e, fontsize=14)
tab_ax = fig.add_axes([0.05, 0.08, 0.4, 0.4]); tab_ax.axis("off")
tab_ax.table(cellText=[["1.02", "0.11"], ["2.98", "0.07"]], rowLabels=[r"$\beta_0$", r"$\beta_1$"], colLabels=["estimate", r"$\pm$ SE"], loc="center")
p_ax = fig.add_axes([0.55, 0.1, 0.4, 0.8])
x = np.linspace(0, 5, 30)
p_ax.plot(x, 1.02 + 2.98 * x, label=r"$\hat{y} = 1.02 + 2.98x$")
p_ax.plot(x, 1 + 3 * x + np.random.default_rng(170).normal(0, 0.5, 30), ".", label=r"data, $N=30$")
p_ax.legend()
fig.canvas.draw()
print([round(t.get_window_extent().width) > 0 for t in t_ax.texts], len(p_ax.get_legend().get_texts()))
fig.savefig("results_page.png", dpi=50)
