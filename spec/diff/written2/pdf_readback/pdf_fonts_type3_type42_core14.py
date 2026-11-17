# Fonts embedded in PDF by fonttype and pdf.use14corefonts; BaseFont names, subtypes and FontFile streams per configuration.
import io
import re
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt


def fonts(rc):
    with plt.rc_context(rc):
        fig, ax = plt.subplots(figsize=(3, 2))
        ax.set_title("Title AVg")
        ax.text(0.2, 0.5, "serif text", family="serif")
        ax.text(0.2, 0.3, r"$\alpha^2$")
        buf = io.BytesIO()
        fig.savefig(buf, format="pdf", metadata={"CreationDate": None})
        plt.close(fig)
    data = buf.getvalue()
    base = sorted(set(re.sub(rb"^[A-Z]{6}\+", b"", n).decode() for n in re.findall(rb"/BaseFont /([^\s/>]+)", data)))
    subtypes = sorted(set(s.decode() for s in re.findall(rb"/Subtype /(Type3|Type1|TrueType|Type0|CIDFontType2)", data)))
    files = sorted(set(s.decode() for s in re.findall(rb"/(FontFile2|FontFile3|FontFile)\b", data)))
    return base, subtypes, files


for rc in [{"pdf.fonttype": 3}, {"pdf.fonttype": 42}, {"pdf.use14corefonts": True, "font.family": "sans-serif",
                                                       "font.sans-serif": ["Helvetica"]},
           {"pdf.fonttype": 42, "mathtext.fontset": "stix"}]:
    print(rc, fonts(rc))
with plt.rc_context({"pdf.fonttype": 42}):
    fig = plt.figure(figsize=(2, 1))
    fig.text(0.1, 0.5, "ÀÉÎõü €")
    buf = io.BytesIO()
    fig.savefig(buf, format="pdf", metadata={"CreationDate": None})
    data = buf.getvalue()
    print(b"/ToUnicode" in data, len(re.findall(rb"/W \[", data)), b"/CIDToGIDMap" in data)
