# Plain text with special characters: dollars, backslashes, percent, underscores, unicode, tabs and escaped mathtext.
import io
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

samples = ["cost: $5 and $10", r"cost: \$5", "100% done", "file_name_here", "tab\there", "unicode: αβγ ← →",
           "back\\slash", "$x$ and $y$", "one $ sign", "{braces} and ^carets", "emoji-free: ✓ ✗", "a b nbsp"]
fig = plt.figure(figsize=(4, 5), dpi=72)
r = fig.canvas.get_renderer()
texts = [fig.text(0.05, 0.95 - k * 0.075, s) for k, s in enumerate(samples)]
fig.canvas.draw()
for t in texts:
    print(repr(t.get_text()), round(t.get_window_extent(r).width, 1), t._preprocess_math(t.get_text())[1])
with plt.rc_context({"svg.fonttype": "none", "svg.hashsalt": "chars"}):
    buf = io.StringIO()
    fig.savefig(buf, format="svg", metadata={"Date": None})
s = buf.getvalue()
print(s.count("<text"), "&#9;" in s or "\t" in s, "100%" in s)
pdf = io.BytesIO()
fig.savefig(pdf, format="pdf", metadata={"CreationDate": None})
print(len(pdf.getvalue()) > 2000)
for t in texts:
    t.set_parse_math(False)
fig.canvas.draw()
print([round(t.get_window_extent(r).width, 1) for t in texts][7:9])
fig.savefig("special.png")
texts[0].set_text("50% of $x$ is $")
fig.canvas.draw()
print(texts[0]._preprocess_math(texts[0].get_text()), round(texts[0].get_window_extent(r).width, 1))
