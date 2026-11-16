# Export one bar chart in every format the journal accepts and report which formats the backend supports.
import matplotlib
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(4, 3))
ax.bar(["A", "B", "C", "D"], [3, 7, 2, 5], color=["#4c72b0", "#dd8452", "#55a868", "#c44e52"])
ax.set_title("Group sizes")
ax.set_ylabel("count")

supported = fig.canvas.get_supported_filetypes()
for ext in sorted(supported):
    print(ext, "->", supported[ext])

for ext in ["png", "pdf", "svg", "eps", "ps", "jpg", "tiff", "webp"]:
    fig.savefig(f"groups.{ext}")
    print("wrote", ext)

print(matplotlib.get_backend())
print(fig.canvas.get_default_filetype())
