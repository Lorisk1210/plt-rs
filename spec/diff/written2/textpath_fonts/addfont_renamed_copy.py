# Register a font file with fontManager.addfont from a private copy, then select it by family name in text and rcParams.
import os
import shutil
import tempfile
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib import font_manager as fm

src = os.path.join(matplotlib.get_data_path(), "fonts", "ttf", "DejaVuSerif-Bold.ttf")
before = len(fm.fontManager.ttflist)
with tempfile.TemporaryDirectory(dir=".") as d:
    copy = os.path.join(d, "MyHeadline.ttf")
    shutil.copyfile(src, copy)
    fm.fontManager.addfont(copy)
    added = fm.fontManager.ttflist[-1]
    print(len(fm.fontManager.ttflist) - before, added.name, added.weight, added.style, os.path.basename(added.fname))
    fp = fm.FontProperties(family="DejaVu Serif", weight="bold")
    print(os.path.basename(fm.findfont(fp)))
    fig, ax = plt.subplots(figsize=(4, 2))
    t = ax.text(0.1, 0.5, "Headline", fontproperties=fm.FontProperties(fname=copy, size=20))
    fig.canvas.draw()
    print(os.path.basename(t.get_fontproperties().get_file()), round(t.get_window_extent().width, 2))
    with plt.rc_context({"font.family": "serif", "font.serif": ["DejaVu Serif"], "font.weight": "bold"}):
        fig2, ax2 = plt.subplots(figsize=(4, 2))
        t2 = ax2.set_title("rc headline")
        fig2.canvas.draw()
        print(t2.get_fontfamily(), t2.get_fontweight(), round(t2.get_window_extent().width, 2))
        fig2.savefig("rc_headline.png")
    fig.savefig("headline.png")
    try:
        fm.fontManager.addfont(os.path.join(d, "missing.ttf"))
    except (FileNotFoundError, OSError) as e:
        print(type(e).__name__)
fm.fontManager.ttflist.remove(added)
print(len(fm.fontManager.ttflist) == before)
fm.fontManager._findfont_cached.cache_clear()
