# SVG metadata (Title, Creator, Keywords) written into the RDF block, read back with ElementTree.
import xml.etree.ElementTree as ET
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(3, 2))
ax.plot([0, 1, 2], [0, 1, 0])
ax.set_title("metadata")
meta = {"Title": "Tent function", "Creator": "lab script", "Keywords": ["tent", "demo"], "Date": None}
with plt.rc_context({"svg.hashsalt": "m"}):
    fig.savefig("meta.svg", metadata=meta)
root = ET.parse("meta.svg").getroot()
ns = {"dc": "http://purl.org/dc/elements/1.1/"}
print(root.find(".//dc:title", ns).text)
print(root.find(".//dc:creator//dc:title", ns).text if root.find(".//dc:creator//dc:title", ns) is not None else None)
print([e.text for e in root.findall(".//dc:subject//*", ns) if e.text])
