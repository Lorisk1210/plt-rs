# SVG metadata: Title, Date, Creator, Keywords, Description, License; the RDF block they produce, and a bad key.
import io
import xml.etree.ElementTree as ET
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(2, 2))
ax.plot([0, 1])
meta = {"Title": "A test", "Date": "2024-01-02", "Creator": "written2", "Keywords": ["a", "b"],
        "Description": "desc", "Rights": "CC0", "Contributor": ["X", "Y"], "Publisher": "pub", "Language": "en"}
buf = io.BytesIO()
fig.savefig(buf, format="svg", metadata=meta)
root = ET.fromstring(buf.getvalue())
DC = "{http://purl.org/dc/elements/1.1/}"
found = {}
for el in root.iter():
    if el.tag.startswith(DC):
        name = el.tag[len(DC):]
        text = " ".join(t.strip() for t in el.itertext() if t.strip())
        found.setdefault(name, text)
for k in sorted(found):
    print(k, repr(found[k]))
title = root.find("{http://www.w3.org/2000/svg}title")
print(None if title is None else title.text)
buf2 = io.BytesIO()
fig.savefig(buf2, format="svg", metadata={"Date": None, "Creator": None, "Format": None, "Type": None})
print(b"<metadata>" in buf2.getvalue(), buf2.getvalue()[:38])
try:
    fig.savefig(io.BytesIO(), format="svg", metadata={"Colour": "red"})
except ValueError as e:
    print("ValueError", str(e)[:70])
try:
    fig.savefig(io.BytesIO(), format="svg", metadata={"Date": 2024})
except (TypeError, ValueError) as e:
    print(type(e).__name__, str(e)[:70])
