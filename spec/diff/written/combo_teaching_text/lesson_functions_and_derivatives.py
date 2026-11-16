# Teaching sheet: a function, its derivative and tangent lines at chosen points, with mathtext labels and a legend table.
import numpy as np
import matplotlib.pyplot as plt

x = np.linspace(-2, 3, 300)
f = lambda t: t ** 3 - 3 * t ** 2 + 2
df = lambda t: 3 * t ** 2 - 6 * t
fig, (a, b) = plt.subplots(2, 1, figsize=(7, 8), sharex=True)
a.plot(x, f(x), label=r"$f(x) = x^3 - 3x^2 + 2$")
for x0, c in [(-1, "C1"), (1, "C2"), (2.5, "C3")]:
    a.plot(x, f(x0) + df(x0) * (x - x0), "--", color=c, lw=1, label=rf"tangent at $x_0={x0}$")
    a.plot(x0, f(x0), "o", color=c)
a.set_ylim(-6, 8)
a.legend(fontsize=8)
b.plot(x, df(x), color="k", label=r"$f'(x) = 3x^2 - 6x$")
b.axhline(0, color="gray", lw=0.5)
b.fill_between(x, df(x), 0, where=df(x) < 0, alpha=0.2, label="decreasing")
b.legend()
print([round(f(t), 3) for t in (-1, 1, 2.5)], [t.get_text() for t in a.get_legend().get_texts()][0])
fig.savefig("calculus_lesson.png", dpi=50)
