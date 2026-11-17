# A line of 10^6 points with path simplification on and off and with markevery; draw times not printed, only output facts.
import io
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

n = 1_000_000
x = np.linspace(0, 100, n)
y = np.sin(x) + 0.001 * np.cos(997 * x)
fig, ax = plt.subplots(figsize=(6, 3), dpi=80)
ln, = ax.plot(x, y, lw=0.5)
fig.canvas.draw()
print(ln.get_xydata().shape, [round(v, 3) for v in ax.get_xlim()], [round(v, 3) for v in ax.get_ylim()])
tp, aff = ln._get_transformed_path().get_transformed_path_and_affine()
print(len(tp.vertices))
for simplify in (True, False):
    with plt.rc_context({"path.simplify": simplify, "path.simplify_threshold": 0.111}):
        buf = io.BytesIO()
        fig.savefig(buf, format="png")
        img = plt.imread(io.BytesIO(buf.getvalue()))
        print(simplify, img.shape, int((img[..., :3].sum(axis=2) < 2.5).sum()) > 1000)
ln.set_marker("o")
ln.set_markevery(50_000)
ln.set_markersize(3)
fig.canvas.draw()
print(ln.get_markevery(), ln.get_marker())
with plt.rc_context({"agg.path.chunksize": 10_000}):
    fig.savefig("million.png")
print(plt.rcParams["agg.path.chunksize"])
svg = io.BytesIO()
small = plt.figure(figsize=(3, 2))
small.add_subplot().plot(x[::100], y[::100])
small.savefig(svg, format="svg", metadata={"Date": None})
print(svg.getvalue().count(b"L "), len(svg.getvalue()) > 10_000)
