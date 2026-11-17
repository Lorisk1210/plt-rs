# PS fonts: ps.fonttype 3 versus 42 and ps.useafm with a core font; which font resources each document defines.
import io
import re
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt


def fonts(rc, text="Hello AVg"):
    with plt.rc_context(rc):
        fig = plt.figure(figsize=(2, 1))
        fig.text(0.1, 0.5, text)
        fig.text(0.1, 0.2, r"$\sum x^2$")
        buf = io.BytesIO()
        fig.savefig(buf, format="ps")
        plt.close(fig)
    t = buf.getvalue().decode("latin-1")
    return (sorted(set(re.findall(r"/FontName /(\S+) def", t))), "/FontType 3" in t, "/FontType 42" in t,
            sorted(set(re.findall(r"^/(\S+) findfont", t, re.M)))[:4], len(t) // 1000)


print(fonts({"ps.fonttype": 3}))
print(fonts({"ps.fonttype": 42}))
print(fonts({"ps.useafm": True, "font.family": "sans-serif", "font.sans-serif": ["Helvetica"]}))
print(fonts({"ps.fonttype": 42, "mathtext.fontset": "cm"}))
print(fonts({"ps.fonttype": 3}, text="ÀÉÎ €"))
with plt.rc_context({"ps.fonttype": 42}):
    fig = plt.figure(figsize=(2, 1))
    fig.text(0.1, 0.5, "abc", family="DejaVu Serif")
    buf = io.BytesIO()
    fig.savefig(buf, format="eps")
    t = buf.getvalue().decode("latin-1")
    print(t.count("/sfnts"), sorted(set(re.findall(r"/FontName /(\S+) def", t))))
