# Save an SVG with gids on bars, then post-process the XML: read each bar's path and style and count elements by gid.
import io
import xml.etree.ElementTree as ET
import matplotlib.pyplot as plt

plt.rcParams["svg.hashsalt"] = "post"
fig, ax = plt.subplots(figsize=(4, 3))
bars = ax.bar(["x", "y", "z"], [3, 5, 2], color=["C0", "C1", "C2"])
for b, n in zip(bars, "xyz"):
    b.set_gid(f"bar_{n}")
buf = io.StringIO()
fig.savefig(buf, format="svg", metadata={"Date": None})
root = ET.fromstring(buf.getvalue())
ns = {"s": "http://www.w3.org/2000/svg"}
for n in "xyz":
    g = root.find(f".//s:g[@id='bar_{n}']", ns)
    path = g.find("s:path", ns)
    print(n, path.get("style"), len(path.get("d").split()))
print(len(root.findall(".//s:g", ns)) > 10)
