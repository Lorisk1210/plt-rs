# PdfPages with keep_empty, metadata on open, attach_note and an empty-figure page.
import matplotlib.pyplot as plt
from matplotlib.backends.backend_pdf import PdfPages

with PdfPages("notes.pdf", metadata={"Title": "Notes", "CreationDate": None}) as pdf:
    for k in range(3):
        fig, ax = plt.subplots(figsize=(4, 3))
        ax.plot(range(k + 2))
        ax.set_title(f"page {k + 1}")
        pdf.attach_note(f"note for page {k + 1}")
        pdf.savefig(fig)
        plt.close(fig)
    blank = plt.figure(figsize=(4, 3))
    pdf.savefig(blank)
    print(pdf.get_pagecount())
with open("notes.pdf", "rb") as f:
    print(f.read().count(b"/Type /Page\n") > 0)
