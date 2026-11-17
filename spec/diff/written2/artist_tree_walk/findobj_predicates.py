# findobj with classes, predicates and include_self on figures, axes and legends; counts and identities of what it finds.
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.lines import Line2D
from matplotlib.text import Text
from matplotlib.patches import Patch, Rectangle

fig, axs = plt.subplots(1, 2, figsize=(6, 3))
axs[0].plot([0, 1], [0, 1], "r-", label="red")
axs[0].plot([0, 1], [1, 0], "b--", label="blue")
axs[1].bar([0, 1, 2], [1, 2, 3])
leg = axs[0].legend()
fig.suptitle("find")
print(len(fig.findobj(Line2D)), len(fig.findobj(Text)), len(fig.findobj(Patch)), len(fig.findobj(Rectangle)))
red = fig.findobj(lambda a: isinstance(a, Line2D) and a.get_color() == "r")
print(len(red), [type(a).__name__ for a in red])
dashed = fig.findobj(lambda a: hasattr(a, "get_linestyle") and a.get_linestyle() == "--")
print(len(dashed), sorted({type(a).__name__ for a in dashed}))
texts = [t.get_text() for t in fig.findobj(Text) if t.get_text()]
print(sorted(texts))
print(len(axs[0].findobj(include_self=False)), len(axs[0].findobj(include_self=True)))
print(leg.findobj(Text)[0].get_text() if leg.findobj(Text) else None, len(leg.findobj(Line2D)))
print(fig.findobj(match=lambda a: False), len(fig.findobj(match=None)) == len(fig.findobj()))
visible = fig.findobj(lambda a: a.get_visible())
print(len(visible) <= len(fig.findobj()))
for a in fig.findobj(Line2D):
    a.set_linewidth(3)
print(sorted({a.get_linewidth() for a in fig.findobj(Line2D)}))
print(len(axs[1].findobj(Rectangle)), len(axs[1].patches))
fig.savefig("findobj.png")
