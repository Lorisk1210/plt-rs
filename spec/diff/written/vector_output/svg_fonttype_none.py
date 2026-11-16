# Editable text in SVG for Inkscape: svg.fonttype 'none' vs 'path'; count <text> elements.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3))
ax.plot([1, 2, 3], [2, 1, 3])
ax.set_title("Editable title")
for fonttype in ["none", "path"]:
    with plt.rc_context({"svg.fonttype": fonttype, "svg.hashsalt": "x"}):
        fig.savefig(f"text_{fonttype}.svg", metadata={"Date": None})
    with open(f"text_{fonttype}.svg") as f:
        content = f.read()
    print(fonttype, content.count("<text"), "font-family" in content)
fig.savefig("svg_fonttype_none_check.png", dpi=40)
print(fig.get_size_inches())
