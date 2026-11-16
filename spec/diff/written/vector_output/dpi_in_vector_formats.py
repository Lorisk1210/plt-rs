# dpi in vector output only affects rasterized parts; check sizes reported in the SVG header.
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 2))
ax.plot([1, 2])
for dpi in (72, 300):
    fig.savefig(f"dpi_{dpi}.svg", dpi=dpi, metadata={"Date": None})
    with open(f"dpi_{dpi}.svg") as f:
        header = [l for l in f.read().splitlines() if l.startswith("<svg")][0]
    print(dpi, header.split('width="')[1].split('"')[0], header.split('height="')[1].split('"')[0])
fig.savefig("dpi.pdf", dpi=300, metadata={"CreationDate": None})
print("pdf saved")
fig.savefig("dpi_in_vector_formats_check.png", dpi=40)
print(fig.get_size_inches())
