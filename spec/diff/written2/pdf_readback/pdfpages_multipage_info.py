# PdfPages with several pages of different sizes, infodict entries, attach_note and get_pagecount; parse page boxes back.
import re
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.backends.backend_pdf import PdfPages

with PdfPages("report.pdf", metadata={"Title": "Report", "Author": "written2", "CreationDate": None}) as pdf:
    for k, size in enumerate([(4, 3), (6, 4), (3, 5)]):
        fig, ax = plt.subplots(figsize=size)
        ax.plot(range(5), [v * (k + 1) for v in range(5)])
        ax.set_title(f"page {k + 1}")
        pdf.attach_note(f"note for page {k + 1}", positionRect=[10, 10, 30, 30])
        pdf.savefig(fig)
        plt.close(fig)
    d = pdf.infodict()
    d["Subject"] = "testing"
    d["Keywords"] = "pdf pages"
    print(pdf.get_pagecount(), sorted(d))
with open("report.pdf", "rb") as f:
    data = f.read()
print(data[:8], data.rstrip().endswith(b"%%EOF"))
print(len(re.findall(rb"/Type\s*/Page\b", data)), re.findall(rb"/Count (\d+)", data))
print([tuple(float(v) for v in m.split()) for m in re.findall(rb"/MediaBox \[\s*([0-9. ]+)\]", data)])
print(len(re.findall(rb"/Subtype\s*/Text", data)), b"note for page 2" in data)
for key in (b"Title", b"Author", b"Subject", b"Keywords", b"Producer", b"Creator", b"CreationDate"):
    m = re.search(rb"/" + key + rb" \(([^)]*)\)", data)
    value = None if m is None else m.group(1).decode()
    # The software's own name in Producer and Creator: printed as present.
    print(key.decode(), value is not None if key in (b"Producer", b"Creator") else value[:40] if value else None)
with PdfPages("empty.pdf", keep_empty=False) as pdf:
    pass
import os
print(os.path.exists("empty.pdf"))
