# Line styles in SVG: dash arrays scaled by linewidth, caps and joins, alpha and colour attributes read back per path.
import io
import re
import xml.etree.ElementTree as ET
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

plt.rcParams["svg.hashsalt"] = "dash"
fig, ax = plt.subplots(figsize=(4, 4))
specs = [dict(ls="--", lw=1), dict(ls="--", lw=3), dict(ls=":", lw=2, dash_capstyle="round"), dict(ls="-.", lw=1.5),
         dict(ls=(0, (5, 2, 1, 2)), lw=1), dict(ls=(3, (4, 4)), lw=2), dict(ls="-", lw=4, solid_capstyle="projecting",
         solid_joinstyle="bevel"), dict(ls="-", lw=2, alpha=0.4, color="#123456")]
for k, kw in enumerate(specs):
    ax.plot([0, 1, 1.5], [k, k + 0.5, k], gid=f"line{k}", **kw)
ax.set_axis_off()
buf = io.BytesIO()
fig.savefig(buf, format="svg", metadata={"Date": None})
root = ET.fromstring(buf.getvalue())
S = "{http://www.w3.org/2000/svg}"
for k in range(len(specs)):
    g = next(e for e in root.iter(S + "g") if e.get("id") == f"line{k}")
    path = next(g.iter(S + "path"))
    style = dict(part.split(": ", 1) for part in path.get("style").split("; "))
    print(k, {key: style[key] for key in sorted(style) if key.startswith("stroke")})
with plt.rc_context({"lines.scale_dashes": False}):
    buf2 = io.BytesIO()
    fig2, ax2 = plt.subplots(figsize=(2, 2))
    ax2.plot([0, 1], ls="--", lw=4)
    fig2.savefig(buf2, format="svg", metadata={"Date": None})
    print(re.findall(r"stroke-dasharray: ([0-9., ]+)", buf2.getvalue().decode()))
print(plt.rcParams["lines.dashed_pattern"], plt.rcParams["lines.dotted_pattern"])
