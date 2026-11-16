# Journal requirement: TrueType (Type 42) fonts embedded in the PDF instead of Type 3.
import matplotlib.pyplot as plt

for ft in (3, 42):
    with plt.rc_context({"pdf.fonttype": ft}):
        fig, ax = plt.subplots(figsize=(3, 2))
        ax.plot([0, 1], [0, 1])
        ax.set_xlabel("label")
        fig.savefig(f"fonts_{ft}.pdf", metadata={"CreationDate": None})
        plt.close(fig)
    with open(f"fonts_{ft}.pdf", "rb") as f:
        data = f.read()
    print(ft, b"/Subtype /Type3" in data, b"/FontFile2" in data)
