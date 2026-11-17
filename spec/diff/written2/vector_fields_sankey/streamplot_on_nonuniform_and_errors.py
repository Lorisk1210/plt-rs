# streamplot input validation: uneven grids, mismatched shapes, unequal spacing tolerance, and 1-D vs 2-D x/y inputs.
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3), dpi=40)
x = np.linspace(0, 1, 10)
y = np.linspace(0, 2, 15)
u = np.ones((15, 10))
v = np.zeros((15, 10))


def attempt(label, *args, **kw):
    try:
        sp = ax.streamplot(*args, density=0.3, **kw)
        print(label, "ok", len(sp.lines.get_segments()) > 0)
    except Exception as e:
        print(label, type(e).__name__, str(e)[:60])


attempt("1-D grid", x, y, u, v)
X, Y = np.meshgrid(x, y)
attempt("2-D grid", X, Y, u, v)
attempt("uneven x", x ** 2, y, u, v)
attempt("decreasing x", x[::-1], y, u, v)
attempt("shape mismatch", x, y, u.T, v.T)
attempt("u v mismatch", x, y, u, v[:-1])
attempt("bad 2-D x", X + np.linspace(0, 0.01, 15)[:, None], Y, u, v)
attempt("tiny grid", x[:2], y[:2], u[:2, :2], v[:2, :2])
attempt("zero field", x, y, u * 0, v)
attempt("nan field", x, y, np.where(X > 0.5, np.nan, u), v)
attempt("linewidth array", x, y, u, v, linewidth=np.ones((15, 10)))
attempt("linewidth wrong", x, y, u, v, linewidth=np.ones((3, 3)))
attempt("color wrong", x, y, u, v, color=np.ones((3, 3)))
attempt("minlength", x, y, u, v, minlength=0.9)
fig.savefig("stream_inputs.png")
print(len(ax.collections), len(ax.patches))
