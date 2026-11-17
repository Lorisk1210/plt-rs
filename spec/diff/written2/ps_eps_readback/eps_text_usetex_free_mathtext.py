# Mathtext in EPS without TeX: the glyphs of each fontset drawn as font text, the bounding box with a tall expression.
import io
import re
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

for fontset in ("dejavusans", "cm", "stix"):
    with plt.rc_context({"mathtext.fontset": fontset, "ps.fonttype": 3}):
        fig = plt.figure(figsize=(3, 1.5))
        fig.text(0.1, 0.3, r"$\int_0^1 \frac{x^2}{\sqrt{1+x}}\,dx$", fontsize=14)
        buf = io.BytesIO()
        fig.savefig(buf, format="eps", bbox_inches="tight")
        t = buf.getvalue().decode("latin-1")
        fonts = sorted(set(re.findall(r"/FontName /(\S+) def", t)))
        bb = re.search(r"%%BoundingBox: ([-\d ]+)", t).group(1)
        print(fontset, fonts, bb, t.count("glyphshow") + t.count(" show"))
        plt.close(fig)
with plt.rc_context({"text.usetex": False}):
    fig = plt.figure(figsize=(2, 1))
    fig.text(0.1, 0.5, r"$a \neq b$")
    buf = io.BytesIO()
    fig.savefig(buf, format="eps")
    print(len(buf.getvalue()) > 1000, plt.rcParams["text.usetex"])
for size in (8, 16, 32):
    fig = plt.figure(figsize=(3, 1.5))
    fig.text(0.1, 0.3, r"$\sum_{i=1}^{n} i$", fontsize=size)
    buf = io.BytesIO()
    fig.savefig(buf, format="eps", bbox_inches="tight", pad_inches=0.05)
    print(size, re.search(r"%%BoundingBox: ([-\d ]+)", buf.getvalue().decode("latin-1")).group(1))
    plt.close(fig)
