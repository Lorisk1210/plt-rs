# Artists that open renderer groups with gids, saved as SVG; the program parses the SVG and lists the groups it made.
import io
import xml.etree.ElementTree as ET
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.artist import Artist
from matplotlib.path import Path
from matplotlib.transforms import IdentityTransform


class Grouped(Artist):
    def __init__(self, name, corner):
        super().__init__()
        self.name = name
        self.corner = corner

    def draw(self, renderer):
        renderer.open_group(self.name, gid=f"grp-{self.name}")
        x, y = self.axes.transAxes.transform(self.corner)
        path = Path([(x, y), (x + 30, y), (x + 30, y + 20), (x, y + 20), (x, y)], closed=True)
        gc = renderer.new_gc()
        gc.set_foreground("k")
        gc.set_url(f"https://example.org/{self.name}")
        renderer.draw_path(gc, path, IdentityTransform(), rgbFace=(0.2, 0.6, 0.9, 1.0))
        gc.restore()
        renderer.close_group(self.name)


plt.rcParams["svg.hashsalt"] = "groups"
fig, ax = plt.subplots(figsize=(4, 3))
for k, name in enumerate(["alpha", "beta", "gamma"]):
    ax.add_artist(Grouped(name, (0.1 + 0.3 * k, 0.4)))
buf = io.StringIO()
fig.savefig(buf, format="svg", metadata={"Date": None})
root = ET.fromstring(buf.getvalue())
ns = {"s": "http://www.w3.org/2000/svg", "x": "http://www.w3.org/1999/xlink"}
ids = [g.get("id") for g in root.iter("{http://www.w3.org/2000/svg}g") if g.get("id", "").startswith("grp-")]
print(ids)
links = [a.get("{http://www.w3.org/1999/xlink}href") for a in root.iter("{http://www.w3.org/2000/svg}a")]
print(links)
fig.savefig("groups.png")
