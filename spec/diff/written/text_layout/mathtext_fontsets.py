# The same formula in each mathtext fontset (dejavusans, cm, stix, stixsans, dejavuserif).
import matplotlib.pyplot as plt

formula = r"$\alpha^2 + \sum_{k=0}^{\infty} \frac{x^k}{k!} = \sqrt{\beta}\,\mathbf{v}$"
fig, ax = plt.subplots(figsize=(6, 4))
for i, fs in enumerate(["dejavusans", "dejavuserif", "cm", "stix", "stixsans"]):
    with plt.rc_context({"mathtext.fontset": fs}):
        ax.text(0.02, 0.88 - i * 0.19, f"{fs}: " + formula, fontsize=13, transform=ax.transAxes)
ax.axis("off")
fig.savefig("fontsets.png")
print(plt.rcParams["mathtext.fontset"])
fig.savefig("mathtext_fontsets_small.png", dpi=50)
print(fig.get_size_inches())
