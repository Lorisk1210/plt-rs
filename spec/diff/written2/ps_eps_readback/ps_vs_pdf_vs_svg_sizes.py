# One scene saved to PS, EPS, PDF and SVG at several complexities; relative document sizes and element counts as it grows.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

plt.rcParams.update({"svg.hashsalt": "sizes", "pdf.compression": 6})
for n in (10, 100, 1000):
    fig, ax = plt.subplots(figsize=(3, 2))
    x = np.linspace(0, 1, n)
    ax.plot(x, np.sin(20 * x))
    ax.scatter(x[::max(1, n // 20)], np.cos(20 * x[::max(1, n // 20)]), s=4)
    sizes = {}
    for fmt in ("ps", "eps", "pdf", "svg"):
        buf = io.BytesIO()
        kw = {"metadata": {"CreationDate": None}} if fmt == "pdf" else {"metadata": {"Date": None}} if fmt == "svg" else {}
        fig.savefig(buf, format=fmt, **kw)
        sizes[fmt] = len(buf.getvalue())
    print(n, sizes["svg"] > sizes["pdf"], sizes["ps"] > sizes["pdf"], abs(sizes["ps"] - sizes["eps"]) < 2000)
    print(n, sorted(sizes, key=sizes.get))
    plt.close(fig)
with plt.rc_context({"path.simplify": False}):
    fig, ax = plt.subplots(figsize=(3, 2))
    ax.plot(np.linspace(0, 1, 5000), np.zeros(5000))
    a, b = io.BytesIO(), io.BytesIO()
    fig.savefig(a, format="svg", metadata={"Date": None})
    plt.rcParams["path.simplify"] = True
    fig.savefig(b, format="svg", metadata={"Date": None})
    print(a.getvalue().count(b"L "), b.getvalue().count(b"L "))
