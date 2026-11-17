# SVG of polar and 3-D axes: group ids per axes, number of paths per group and the clip path of the polar patch.
import io
import xml.etree.ElementTree as ET
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

plt.rcParams["svg.hashsalt"] = "proj"
fig = plt.figure(figsize=(6, 3))
pax = fig.add_subplot(1, 2, 1, projection="polar")
pax.plot(np.linspace(0, 2 * np.pi, 50), np.linspace(0, 1, 50))
ax3 = fig.add_subplot(1, 2, 2, projection="3d")
ax3.plot([0, 1], [0, 1], [0, 1])
ax3.scatter([0.5], [0.5], [0.5])
buf = io.BytesIO()
fig.savefig(buf, format="svg", metadata={"Date": None})
root = ET.fromstring(buf.getvalue())
S = "{http://www.w3.org/2000/svg}"
top = [g for g in root.find(S + "g")]
print([g.get("id") for g in top])
for g in top:
    if g.get("id", "").startswith("axes"):
        paths = list(g.iter(S + "path"))
        groups = [c.get("id") for c in g if c.get("id")]
        print(g.get("id"), len(paths), groups[:6])
clips = list(root.iter(S + "clipPath"))
print(len(clips), [len(list(c)) for c in clips])
polar_clip = [c for c in clips if next(iter(c)).tag == S + "path"]
print(len(polar_clip))
print(sorted({e.get("id").rstrip("0123456789_") for e in root.iter() if e.get("id") and not e.get("id").startswith(("m", "p"))})[:12])
