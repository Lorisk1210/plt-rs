# Scatter of 10^5 points with per-point colours and sizes, rasterized in a PDF, and a hexbin of the same data.
import io
import re
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt


def splitmix(seed, n):
    out = np.empty(n, dtype=np.uint64)
    state = np.uint64(seed)
    with np.errstate(over="ignore"):
        for i in range(n):
            state = state + np.uint64(0x9E3779B97F4A7C15)
            z = state
            z = (z ^ (z >> np.uint64(30))) * np.uint64(0xBF58476D1CE4E5B9)
            z = (z ^ (z >> np.uint64(27))) * np.uint64(0x94D049BB133111EB)
            out[i] = z ^ (z >> np.uint64(31))
    return out / np.float64(2 ** 64)


n = 100_000
u = splitmix(7, 2 * n).reshape(2, n)
x = np.sqrt(-2 * np.log(u[0] + 1e-12)) * np.cos(2 * np.pi * u[1])
y = np.sqrt(-2 * np.log(u[0] + 1e-12)) * np.sin(2 * np.pi * u[1])
fig, (a1, a2) = plt.subplots(1, 2, figsize=(7, 3), dpi=60)
sc = a1.scatter(x, y, c=np.hypot(x, y), s=1 + 3 * u[1], cmap="plasma", rasterized=True, linewidths=0)
hb = a2.hexbin(x, y, gridsize=30, cmap="Blues", mincnt=1)
fig.canvas.draw()
print(sc.get_offsets().shape, round(float(sc.get_array().max()), 4), len(sc.get_sizes()))
print(hb.get_array().shape, int(hb.get_array().sum()), int(hb.get_array().max()))
buf = io.BytesIO()
fig.savefig(buf, format="pdf", metadata={"CreationDate": None})
print(len(re.findall(rb"/Subtype /Image", buf.getvalue())), len(buf.getvalue()) < 2_000_000)
fig.savefig("scatter.png")
print([round(v, 3) for v in a1.get_xlim()], [round(v, 3) for v in a2.get_ylim()])
