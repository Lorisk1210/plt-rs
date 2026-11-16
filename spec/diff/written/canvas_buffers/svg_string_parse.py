# Produce SVG in memory and inspect it with ElementTree (count paths, read width/height).
import io
import xml.etree.ElementTree as ET
import matplotlib.pyplot as plt

plt.rcParams["svg.hashsalt"] = "fixed-salt"
fig, ax = plt.subplots(figsize=(4, 3))
ax.plot([0, 1, 2], [1, 0, 1], gid="my-line")
ax.set_title("svg")
buf = io.StringIO()
fig.savefig(buf, format="svg", metadata={"Date": None})
root = ET.fromstring(buf.getvalue())
ns = {"svg": "http://www.w3.org/2000/svg"}
print(root.get("width"), root.get("height"), root.get("viewBox"))
print(root.find(".//svg:g[@id='my-line']", ns) is not None)
print(len(root.findall(".//svg:path", ns)) > 5)
