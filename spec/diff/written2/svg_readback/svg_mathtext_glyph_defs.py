# Mathtext in SVG as paths: glyph definitions per font (DejaVu, Cmr, STIX), reuse of each glyph and rects for fraction bars.
import io
import re
import collections
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

for fontset in ("dejavusans", "cm", "stix"):
    with plt.rc_context({"mathtext.fontset": fontset, "svg.hashsalt": "math", "svg.fonttype": "path"}):
        fig = plt.figure(figsize=(3, 1))
        fig.text(0.05, 0.3, r"$\frac{a+a}{\sqrt{b}} \alpha\alpha$", fontsize=16)
        buf = io.StringIO()
        fig.savefig(buf, format="svg", metadata={"Date": None})
        s = buf.getvalue()
        glyph_ids = re.findall(r'<path id="([A-Za-z0-9_-]+)"', s)
        families = collections.Counter(g.split("-")[0] for g in glyph_ids)
        uses = collections.Counter(re.findall(r'xlink:href="#([A-Za-z0-9_-]+)"', s))
        print(fontset, dict(sorted(families.items())), sorted(uses.values(), reverse=True)[:3])
        print(fontset, s.count("<rect"), len(glyph_ids))
        plt.close(fig)
with plt.rc_context({"svg.fonttype": "none"}):
    fig = plt.figure(figsize=(3, 1))
    fig.text(0.05, 0.3, r"$x^2 + \beta$", fontsize=16)
    buf = io.StringIO()
    fig.savefig(buf, format="svg", metadata={"Date": None})
    s = buf.getvalue()
    print(s.count("<text"), s.count("<tspan"), "β" in s or "&#946;" in s)
    with plt.rc_context({"svg.fonttype": "path"}):
        buf2 = io.StringIO()
        fig.savefig(buf2, format="svg", metadata={"Date": None})
        print(buf2.getvalue().count("<text"), len(re.findall(r'<path id="', buf2.getvalue())))
