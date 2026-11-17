# PostScript DSC header for papertypes and orientations: %%BoundingBox, %%Orientation, %%DocumentPaperSizes and page setup.
import io
import re
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(5, 3))
ax.plot([0, 1], [1, 0])


def header(**kw):
    buf = io.BytesIO()
    fig.savefig(buf, format="ps", **kw)
    text = buf.getvalue().decode("latin-1")
    head = text.split("%%EndComments")[0]
    keys = {}
    for line in head.splitlines():
        m = re.match(r"%%(\w+):\s*(.*)", line)
        if m and m.group(1) not in ("Creator", "CreationDate", "Title"):
            keys[m.group(1)] = m.group(2)
    return keys


for kw in [{}, {"papertype": "a4"}, {"papertype": "letter", "orientation": "landscape"}, {"papertype": "figure"},
           {"papertype": "b5", "orientation": "portrait"}]:
    print(kw, header(**kw))
with plt.rc_context({"ps.papersize": "legal"}):
    print("rc legal", header())
try:
    header(papertype="a99")
except ValueError as e:
    print("ValueError", str(e)[:60])
buf = io.BytesIO()
fig.savefig(buf, format="ps", metadata={"Creator": "written2", "Title": "ps header"})
text = buf.getvalue().decode("latin-1")
print([l for l in text.splitlines()[:12] if l.startswith(("%%Creator", "%%Title", "%!PS"))])
