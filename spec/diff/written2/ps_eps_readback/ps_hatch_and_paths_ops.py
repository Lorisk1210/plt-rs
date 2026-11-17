# Paths in PostScript: moveto/lineto/curveto counts for lines, circles and hatches; clip operators and the procset definitions.
import io
import re
import collections
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.patches import Circle, Rectangle

fig, ax = plt.subplots(figsize=(3, 3))
ax.add_patch(Circle((0.5, 0.5), 0.3, fc="none"))
ax.add_patch(Rectangle((0.1, 0.1), 0.3, 0.2, hatch="xx", fill=False))
ax.plot([0, 1], [0, 1], "k--")
buf = io.BytesIO()
fig.savefig(buf, format="ps")
t = buf.getvalue().decode("latin-1")
body = t.split("%%EndProlog")[-1]
ops = collections.Counter(re.findall(r"(?<=\s)(m|l|c|cl|clipbox|stroke|fill|gsave|grestore|setdash|setlinewidth|"
                                     r"setlinecap|setlinejoin|closepath|newpath)(?=\s)", body))
print(sorted(ops.items()))
print(len(re.findall(r"/H\d+ ", t)), "PaintType" in t, "makepattern" in t)
prolog = t.split("%%BeginProlog")[1].split("%%EndProlog")[0] if "%%BeginProlog" in t else ""
defs = sorted(set(re.findall(r"^/(\w+) \{", prolog, re.M)))
print(defs[:12])
print(t.count("%%Page:"), t.count("showpage"), "%%Trailer" in t, t.rstrip().endswith("%%EOF"))
buf2 = io.BytesIO()
fig.savefig(buf2, format="ps", bbox_inches="tight")
t2 = buf2.getvalue().decode("latin-1")
print(re.search(r"%%BoundingBox: ([-\d ]+)", t2).group(1), len(t2) > 1000)
print(len(prolog) > 100, t.count("%%BeginProlog"))
