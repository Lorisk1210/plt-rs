# Multi-page PDF catalogue: per product a page with price history, a review histogram and a small table.
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.backends.backend_pdf import PdfPages

rng = np.random.default_rng(11)
products = ["kettle", "toaster", "blender", "mixer", "grill", "fryer"]
pages = []
with PdfPages("catalogue.pdf", metadata={"CreationDate": None}) as pdf:
    for name in products:
        price = 50 + np.cumsum(rng.normal(0, 1.2, 52))
        reviews = rng.choice([1, 2, 3, 4, 5], 200, p=[0.05, 0.05, 0.15, 0.35, 0.4])
        fig, (a, b, c) = plt.subplots(1, 3, figsize=(11, 3.5), gridspec_kw={"width_ratios": [2, 1, 1]})
        a.plot(price)
        a.set_title(f"{name}: weekly price")
        counts = np.bincount(reviews, minlength=6)[1:]
        b.barh(["1★", "2★", "3★", "4★", "5★"], counts, color="gold")
        c.axis("off")
        c.table(cellText=[["min", f"{price.min():.2f}"], ["max", f"{price.max():.2f}"], ["rating", f"{reviews.mean():.2f}"]], loc="center")
        pdf.savefig(fig)
        pages.append((name, counts.tolist()))
        plt.close(fig)
    print(pdf.get_pagecount())
print(pages[0])
