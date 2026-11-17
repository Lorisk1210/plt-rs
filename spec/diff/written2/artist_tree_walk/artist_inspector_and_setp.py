# ArtistInspector on several classes: setters, aliases, valid values; setp/getp printing to stdout and returning values.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.artist import ArtistInspector, getp, setp
from matplotlib.lines import Line2D
from matplotlib.patches import Rectangle
from matplotlib.text import Text

for cls in (Line2D, Rectangle, Text):
    insp = ArtistInspector(cls)
    setters = insp.get_setters()
    aliases = insp.get_aliases()
    print(cls.__name__, len(setters) > 30, sorted(setters)[:5], sorted(aliases.get("linewidth", aliases.get("color", set()))))
insp = ArtistInspector(Line2D)
for prop in ("linestyle", "drawstyle", "fillstyle", "solid_capstyle", "antialiased"):
    print(prop, insp.get_valid_values(prop)[:80])
print(insp.aliased_name("lw"), insp.aliased_name("linewidth"))
fig, ax = plt.subplots()
ln, = ax.plot([0, 1])
out = setp(ln, linewidth=3, color="g", ls=":")
print(out)
print(getp(ln, "linewidth"), getp(ln, "lw"), getp(ln, "color"))
setp([ln], "marker", "o")
print(ln.get_marker())
setp(ln, "linestyle")
lines = insp.pprint_setters(prop="linestyle")
print(lines)
print(len(insp.pprint_setters()) > 30)
print(len(insp.properties()) > 30, sorted(insp.properties())[:4])
print(getp(ax, "xlim"), getp(ax.title, "text") == "")
