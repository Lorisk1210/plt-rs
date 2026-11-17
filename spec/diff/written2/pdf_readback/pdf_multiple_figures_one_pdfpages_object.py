# One PdfPages object fed figures from pyplot, Figure() without pyplot, and savefig(pdf, format="pdf"); page order check.
import re
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt
from matplotlib.figure import Figure
from matplotlib.backends.backend_pdf import PdfPages

pdf = PdfPages("mixed.pdf", metadata={"CreationDate": None})
fig1, ax1 = plt.subplots(figsize=(3, 2))
ax1.set_title("first")
pdf.savefig(fig1)
fig2 = Figure(figsize=(4, 2))
fig2.add_subplot().set_title("second")
pdf.savefig(fig2)
fig3, ax3 = plt.subplots(figsize=(2, 2))
ax3.set_title("third")
fig3.savefig(pdf, format="pdf")
plt.figure(fig1.number)
pdf.savefig()
print(pdf.get_pagecount())
pdf.close()
with open("mixed.pdf", "rb") as f:
    data = f.read()
print([tuple(round(float(v)) for v in m.split()) for m in re.findall(rb"/MediaBox \[\s*([0-9. ]+)\]", data)])
try:
    pdf.savefig(fig1)
except Exception as e:
    print(type(e).__name__)
with PdfPages("numbered.pdf", metadata={"CreationDate": None}) as pp:
    for k in range(3):
        f = plt.figure(figsize=(2, 2))
        pp.savefig(f)
    pp.savefig(1)
    print(pp.get_pagecount())
print(sorted(plt.get_fignums()))
