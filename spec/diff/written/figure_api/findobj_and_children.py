# Walk the artist tree: get_children, findobj by type and by predicate to restyle all texts.
import matplotlib.pyplot as plt
from matplotlib.text import Text
from matplotlib.lines import Line2D

fig, (a, b) = plt.subplots(1, 2, figsize=(7, 3))
a.plot([1, 2, 3], label="x")
b.plot([3, 2, 1], label="y")
a.legend()
a.set_title("A")
b.set_title("B")
texts = fig.findobj(Text)
for t in texts:
    t.set_color("darkgreen")
lines = fig.findobj(lambda art: isinstance(art, Line2D) and art.get_label() in ("x", "y"))
print(len(texts) > 10, len(lines), len(a.get_children()) > 5)
fig.savefig("restyled.png")
