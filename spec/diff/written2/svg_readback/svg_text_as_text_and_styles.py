# svg.fonttype none: text elements with font styles, rotation, anchors, multiline tspans and the CSS each carries.
import io
import xml.etree.ElementTree as ET
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

plt.rcParams.update({"svg.fonttype": "none", "svg.hashsalt": "text"})
fig, ax = plt.subplots(figsize=(4, 3))
ax.text(0.1, 0.8, "plain")
ax.text(0.5, 0.5, "bold rotated", weight="bold", rotation=30, ha="center", va="center")
ax.text(0.9, 0.2, "italic right", style="italic", ha="right", color="red", fontsize=14)
ax.text(0.1, 0.3, "two\nlines", family="monospace")
ax.text(0.5, 0.1, "x < y & z", alpha=0.5)
ax.set_axis_off()
buf = io.BytesIO()
fig.savefig(buf, format="svg", metadata={"Date": None})
root = ET.fromstring(buf.getvalue())
S = "{http://www.w3.org/2000/svg}"
for el in root.iter(S + "text"):
    content = "".join(el.itertext())
    style = el.get("style", "")
    keys = sorted(part.split(":")[0].strip() for part in style.split(";") if part.strip())
    print(repr(content), keys, el.get("transform", "")[:30], el.get("x"), el.get("y"))
print(len(list(root.iter(S + "tspan"))))
print(b"x &lt; y &amp; z" in buf.getvalue())
plt.rcParams["svg.fonttype"] = "path"
buf2 = io.BytesIO()
fig.savefig(buf2, format="svg", metadata={"Date": None})
root2 = ET.fromstring(buf2.getvalue())
print(len(list(root2.iter(S + "text"))), len([g for g in root2.iter(S + "g") if g.get("id", "").startswith("text_")]))
