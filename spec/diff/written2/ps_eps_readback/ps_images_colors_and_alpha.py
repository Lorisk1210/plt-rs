# Images, colours and transparency in PostScript: colorimage operators, setrgbcolor counts, and what happens to alpha.
import io
import re
import warnings
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(3, 3))
ax.imshow(np.arange(12).reshape(3, 4), extent=(0, 1, 0, 1), cmap="viridis")
ax.imshow(np.ones((2, 2)), extent=(1, 2, 1, 2), cmap="gray", alpha=0.5)
ax.fill([0, 2, 2], [0, 0, 2], color="red", alpha=0.3)
ax.plot([0, 2], [2, 0], color=(0, 0.5, 0, 0.6), lw=3)
ax.set_xlim(0, 2)
ax.set_ylim(0, 2)
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    buf = io.BytesIO()
    fig.savefig(buf, format="ps")
print(sorted({str(w.message)[:70] for w in caught}))
t = buf.getvalue().decode("latin-1")
print(t.count("colorimage"), t.count(" image"), len(re.findall(r"setrgbcolor", t)) > 0, t.count("setgray") >= 0)
print(re.findall(r"(\d+) (\d+) 8 \[", t)[:3])
with warnings.catch_warnings(record=True) as caught:
    warnings.simplefilter("always")
    buf2 = io.BytesIO()
    fig.savefig(buf2, format="eps")
print(len(caught), buf2.getvalue()[:10])
gray = plt.figure(figsize=(1, 1))
gray.add_axes([0, 0, 1, 1]).imshow(np.eye(3), cmap="gray")
buf3 = io.BytesIO()
gray.savefig(buf3, format="ps")
print(buf3.getvalue().decode("latin-1").count("colorimage"))
