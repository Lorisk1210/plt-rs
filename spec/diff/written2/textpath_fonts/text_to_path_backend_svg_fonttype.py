# The same text saved to SVG as paths and as text, and to PDF/PS with fonttype 3 and 42; compare what each document contains.
import io
import re
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

def build():
    fig, ax = plt.subplots(figsize=(4, 2))
    ax.text(0.1, 0.5, "Glyphs: AVAg 123", fontsize=16)
    ax.set_axis_off()
    return fig

for fonttype in ("path", "none"):
    with plt.rc_context({"svg.fonttype": fonttype, "svg.hashsalt": "fonts"}):
        fig = build()
        buf = io.StringIO()
        fig.savefig(buf, format="svg", metadata={"Date": None})
        s = buf.getvalue()
        print(fonttype, s.count("<text"), len(re.findall(r'<path id="DejaVuSans-', s)), "font-family" in s)
        plt.close(fig)
for fonttype in (3, 42):
    with plt.rc_context({"pdf.fonttype": fonttype, "ps.fonttype": fonttype}):
        fig = build()
        pdf = io.BytesIO()
        fig.savefig(pdf, format="pdf", metadata={"CreationDate": None})
        b = pdf.getvalue()
        print(fonttype, b"/Subtype /Type3" in b, b"/Subtype /TrueType" in b or b"/Subtype /Type0" in b, b"/FontFile2" in b)
        ps = io.BytesIO()
        fig.savefig(ps, format="ps")
        t = ps.getvalue().decode("latin-1")
        print(fonttype, "/FontType 3" in t, "/FontType 42" in t, "/sfnts" in t)
        plt.close(fig)
