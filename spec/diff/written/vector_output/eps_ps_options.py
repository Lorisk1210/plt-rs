# EPS for LaTeX and PS with papertype and orientation options.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3))
ax.bar(["a", "b"], [1, 2])
ax.set_title("postscript")
fig.savefig("figure.eps")
fig.savefig("figure_a4.ps", papertype="a4", orientation="landscape")
plt.rcParams["ps.usedistiller"] = None
fig.savefig("figure_letter.ps", papertype="letter")
for name in ["figure.eps", "figure_a4.ps", "figure_letter.ps"]:
    with open(name, "rb") as f:
        head = f.read(30)
    print(name, head.split(b"\n")[0])
