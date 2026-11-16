# Grid of predicted images with titles coloured green/red by correctness and a bar of per-class accuracy.
import numpy as np
import matplotlib.pyplot as plt

rng = np.random.default_rng(48)
labels = ["zero", "one", "two"]
fig = plt.figure(figsize=(10, 6))
gs = fig.add_gridspec(3, 6)
correct_by_class = np.zeros(3); total = np.zeros(3)
for k in range(12):
    true = k % 3
    pred = true if rng.random() > 0.25 else (true + 1) % 3
    correct_by_class[true] += pred == true; total[true] += 1
    ax = fig.add_subplot(gs[k // 4, k % 4])
    img = np.zeros((12, 12)); img[2:10, 3 + true:5 + true * 2] = 1
    ax.imshow(img + rng.random((12, 12)) * 0.3, cmap="gray_r")
    ax.set_title(f"{labels[pred]}", color="green" if pred == true else "red", fontsize=9)
    ax.axis("off")
axb = fig.add_subplot(gs[:, 4:])
axb.bar(labels, correct_by_class / total, color="C2")
axb.set_ylim(0, 1)
axb.set_title("accuracy")
fig.tight_layout()
print((correct_by_class / total).round(3).tolist())
fig.savefig("predictions.png", dpi=50)
