# Hatch patterns and transparency in PDF: /Pattern resources, ExtGState alpha values, and how they grow with distinct hatches.
import io
import re
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Rectangle


def summary(hatches, alphas, colors):
    fig, ax = plt.subplots(figsize=(4, 2))
    for k, (h, a, c) in enumerate(zip(hatches, alphas, colors)):
        ax.add_patch(Rectangle((k, 0), 0.8, 1, hatch=h, alpha=a, fc=c, ec="k"))
    ax.set_xlim(0, len(hatches))
    buf = io.BytesIO()
    fig.savefig(buf, format="pdf", metadata={"CreationDate": None})
    plt.close(fig)
    data = buf.getvalue()
    return (len(re.findall(rb"/PatternType 1", data)), sorted({float(v) for v in re.findall(rb"/CA ([0-9.]+)", data)}),
            sorted({float(v) for v in re.findall(rb"/ca ([0-9.]+)", data)}), len(re.findall(rb"/Type /ExtGState", data)))


print(summary(["/", "/", "x"], [None, None, None], ["w", "w", "w"]))
print(summary(["/", "\\", "x", "o", "."], [0.2, 0.4, 0.6, 0.8, 1.0], ["r", "g", "b", "c", "m"]))
print(summary(["//", "//", "//"], [0.5, 0.5, 0.5], ["r", "g", "r"]))
print(summary([None, None], [0.3, 0.7], ["k", "k"]))
with plt.rc_context({"hatch.color": "red", "hatch.linewidth": 3}):
    print(summary(["x", "x"], [None, None], ["w", "w"]))
fig, ax = plt.subplots()
ax.bar([0, 1], [1, 2], hatch="//", edgecolor="blue", color="none")
buf = io.BytesIO()
fig.savefig(buf, format="pdf", metadata={"CreationDate": None})
print(len(re.findall(rb"/PatternType 1", buf.getvalue())))
