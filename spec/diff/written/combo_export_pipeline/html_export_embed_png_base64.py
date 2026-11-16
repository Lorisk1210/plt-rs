# Build a small HTML report string embedding two charts as base64 PNGs, and check the HTML structure.
import base64
import io
import numpy as np
import matplotlib.pyplot as plt

def to_b64(fig):
    buf = io.BytesIO()
    fig.savefig(buf, format="png", dpi=50)
    plt.close(fig)
    return base64.b64encode(buf.getvalue()).decode()

f1, a1 = plt.subplots(figsize=(3, 2)); a1.plot([1, 3, 2])
f2, a2 = plt.subplots(figsize=(3, 2)); a2.bar(["a", "b"], [2, 3])
images = [to_b64(f1), to_b64(f2)]
html = "<html><body>" + "".join(f'<img src="data:image/png;base64,{s}"/>' for s in images) + "</body></html>"
with open("report.html", "w") as f:
    f.write(html)
print(html.count("<img"), html.startswith("<html>"), [base64.b64decode(s)[:4] for s in images])
