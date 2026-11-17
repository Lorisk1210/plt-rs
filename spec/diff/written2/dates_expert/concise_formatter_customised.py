# ConciseDateFormatter customised: formats, zero_formats, offset_formats, show_offset; labels over spans from seconds to years.
import datetime
import numpy as np
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
import matplotlib.dates as mdates

base = datetime.datetime(2023, 12, 31, 23, 59, 50)
spans = [datetime.timedelta(seconds=30), datetime.timedelta(minutes=20), datetime.timedelta(hours=10),
         datetime.timedelta(days=12), datetime.timedelta(days=200), datetime.timedelta(days=3000)]
fig, ax = plt.subplots(figsize=(7, 2), dpi=60)
loc = mdates.AutoDateLocator(minticks=3, maxticks=7)
for variant in ("default", "custom"):
    fmt = mdates.ConciseDateFormatter(loc)
    if variant == "custom":
        fmt.formats = ["%y", "%b", "%d", "%H:%M", "%H:%M", "%S.%f"]
        fmt.zero_formats = [""] + fmt.formats[:-1]
        fmt.zero_formats[3] = "%d-%b"
        fmt.offset_formats = ["", "%Y", "%b %Y", "%d %b %Y", "%d %b %Y", "%d %b %Y %H:%M"]
    ax.xaxis.set_major_locator(loc)
    ax.xaxis.set_major_formatter(fmt)
    for span in spans:
        ax.set_xlim(base, base + span)
        fig.canvas.draw()
        print(variant, span, [t.get_text() for t in ax.get_xticklabels()][:5], repr(ax.xaxis.get_offset_text().get_text()))
fmt = mdates.ConciseDateFormatter(loc, show_offset=False)
ax.xaxis.set_major_formatter(fmt)
fig.canvas.draw()
print(repr(ax.xaxis.get_offset_text().get_text()), fmt.format_data_short(mdates.date2num(base)))
conv = mdates.ConciseDateConverter()
print(type(conv.axisinfo(None, ax.xaxis).majfmt).__name__)
fig.savefig("concise.png")
