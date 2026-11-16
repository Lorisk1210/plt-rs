# Render small charts into PNG bytes in memory, read them back with imread, and tile them onto pages of a PdfPages report.
import io
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.backends.backend_pdf import PdfPages

tiles = []
for k in range(6):
    f, ax = plt.subplots(figsize=(2, 1.5), dpi=60)
    ax.plot(np.sin(np.linspace(0, (k + 1) * np.pi, 50)))
    ax.set_title(f"tile {k}", fontsize=8)
    buf = io.BytesIO()
    f.savefig(buf, format="png")
    plt.close(f)
    buf.seek(0)
    tiles.append(plt.imread(buf))
with PdfPages("tiles.pdf", metadata={"CreationDate": None}) as pdf:
    for page in range(2):
        fig, axs = plt.subplots(1, 3, figsize=(8, 2.5))
        for ax, img in zip(axs, tiles[page * 3:page * 3 + 3]):
            ax.imshow(img)
            ax.axis("off")
        pdf.savefig(fig)
        plt.close(fig)
    print(pdf.get_pagecount())
print([t.shape for t in tiles[:2]], round(float(tiles[0][..., :3].mean()), 4))
