# Text in PostScript without a distiller: glyphshow vs show operators, font size scaling, rotation and the char strings drawn.
import io
import re
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

plt.rcParams["ps.usedistiller"] = None
fig = plt.figure(figsize=(4, 2))
fig.text(0.1, 0.7, "Plain text", fontsize=12)
fig.text(0.5, 0.4, "Rotated", rotation=30, fontsize=9)
fig.text(0.1, 0.1, "Mono 123", family="monospace", fontsize=15)
buf = io.BytesIO()
fig.savefig(buf, format="ps")
t = buf.getvalue().decode("latin-1")
body = t.split("%%EndProlog")[-1]
print(body.count("glyphshow"), body.count(" show"), body.count("rotate"))
print(sorted(set(re.findall(r"/(\S+) ([\d.]+) selectfont", body))))
print(re.findall(r"\(([^)]*)\) show", body)[:4])
print(sorted(set(re.findall(r"/(\w+) glyphshow", body)))[:12])
with plt.rc_context({"ps.fonttype": 42}):
    buf2 = io.BytesIO()
    fig.savefig(buf2, format="ps")
    b2 = buf2.getvalue().decode("latin-1").split("%%EndProlog")[-1]
    print(b2.count("glyphshow"), b2.count(" show"), sorted(set(re.findall(r"/(\S+) ([\d.]+) selectfont", b2)))[:3])
print(plt.rcParams["ps.usedistiller"], plt.rcParams["ps.distiller.res"])
fig.texts[0].set_text("(parens) and \\backslash")
buf3 = io.BytesIO()
fig.savefig(buf3, format="ps")
b3 = buf3.getvalue().decode("latin-1")
print(re.findall(r"\(([^\n]*)\) show", b3)[:2], b3.count("glyphshow"))
