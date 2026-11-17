# Mathtext drawn into SVG (fonttype none and path), PDF and PS: glyph references, fonts listed and rules (fraction bars).
import io
import re
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

expr = r"$\frac{\alpha^2}{\sqrt{\beta}} + \sum_i x_i$"


def scene():
    fig = plt.figure(figsize=(3, 1.2))
    fig.text(0.05, 0.3, expr, fontsize=16)
    return fig


for fonttype in ("path", "none"):
    with plt.rc_context({"svg.fonttype": fonttype, "svg.hashsalt": "math"}):
        fig = scene()
        s = io.StringIO()
        fig.savefig(s, format="svg", metadata={"Date": None})
        v = s.getvalue()
        print(fonttype, v.count("<use"), v.count("<text"), v.count("<tspan"), len(re.findall(r'<path id="(DejaVu|Cmr|STIX)', v)))
        plt.close(fig)
for fontset in ("dejavusans", "cm", "stix"):
    with plt.rc_context({"mathtext.fontset": fontset}):
        fig = scene()
        pdf = io.BytesIO()
        fig.savefig(pdf, format="pdf", metadata={"CreationDate": None})
        fonts = sorted(set(re.findall(rb"/BaseFont /([A-Za-z0-9+_-]+)", pdf.getvalue())))
        print(fontset, [f.decode() for f in fonts])
        ps = io.BytesIO()
        fig.savefig(ps, format="ps")
        t = ps.getvalue().decode("latin-1")
        print(fontset, sorted(set(re.findall(r"^%%DocumentSuppliedResources: font (\S+)", t, re.M) +
                                  re.findall(r"^%%\+ font (\S+)", t, re.M))))
        plt.close(fig)
