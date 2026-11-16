# Interactive SVG: set gids and URLs on artists so they can be scripted on a web page.
import matplotlib.pyplot as plt

plt.rcParams["svg.hashsalt"] = "deterministic"
fig, ax = plt.subplots(figsize=(4, 3))
bars = ax.bar(["x", "y", "z"], [3, 5, 2])
for bar, name in zip(bars, "xyz"):
    bar.set_gid(f"bar-{name}")
    bar.set_url(f"https://example.org/{name}")
fig.savefig("links.svg", metadata={"Date": None})
with open("links.svg") as f:
    svg = f.read()
print(svg.count('id="bar-'), svg.count("<a "), "xlink:href" in svg)
