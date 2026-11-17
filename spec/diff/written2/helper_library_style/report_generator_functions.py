# A report generator: functions returning (figure, summary dict) per section, assembled into a multi-page PDF and a PNG index.
import io
import re
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backends.backend_pdf import PdfPages


def section_trend(values):
    fig, ax = plt.subplots(figsize=(4, 3), dpi=60)
    ax.plot(values)
    slope = float(np.polyfit(np.arange(len(values)), values, 1)[0])
    ax.set_title(f"trend {slope:+.3f}/step")
    return fig, {"slope": round(slope, 4)}


def section_distribution(values):
    fig, ax = plt.subplots(figsize=(4, 3), dpi=60)
    counts, edges, _ = ax.hist(values, bins=8)
    return fig, {"mode_bin": int(np.argmax(counts)), "edges": [round(e, 3) for e in edges[:3]]}


def section_table(values):
    fig, ax = plt.subplots(figsize=(4, 2), dpi=60)
    ax.axis("off")
    stats = [["mean", f"{np.mean(values):.3f}"], ["std", f"{np.std(values):.3f}"], ["max", f"{np.max(values):.3f}"]]
    ax.table(cellText=stats, colLabels=["stat", "value"], loc="center")
    return fig, dict(stats)


values = np.cumsum(np.sin(np.arange(60) * 0.37)) + np.arange(60) * 0.05
buf = io.BytesIO()
summaries = []
with PdfPages(buf, metadata={"CreationDate": None, "Title": "report"}) as pdf:
    for section in (section_trend, section_distribution, section_table):
        fig, summary = section(values)
        pdf.savefig(fig)
        fig.savefig(f"{section.__name__}.png")
        plt.close(fig)
        summaries.append((section.__name__, summary))
    pages = pdf.get_pagecount()
for name, summary in summaries:
    print(name, summary)
print(pages, len(re.findall(rb"/Type\s*/Page\b", buf.getvalue())), plt.get_fignums())
