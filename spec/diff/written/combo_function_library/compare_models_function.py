# compare_models(results) helper: bar chart of metrics with error bars plus a table, called for two experiments.
import numpy as np
import matplotlib.pyplot as plt


def compare_models(results, metric_names, title):
    names = list(results)
    x = np.arange(len(metric_names))
    width = 0.8 / len(names)
    fig, (ax, tab) = plt.subplots(2, 1, figsize=(7, 5), gridspec_kw={"height_ratios": [3, 1]})
    for i, name in enumerate(names):
        mean, sd = results[name]
        ax.bar(x + i * width - 0.4 + width / 2, mean, width, yerr=sd, capsize=3, label=name)
    ax.set_xticks(x, metric_names)
    ax.set_ylim(0, 1)
    ax.legend(ncols=len(names), loc="upper center", bbox_to_anchor=(0.5, 1.15))
    tab.axis("off")
    tab.table(cellText=[[f"{m:.2f}" for m in results[n][0]] for n in names], rowLabels=names, colLabels=metric_names, loc="center")
    fig.suptitle(title, y=1.02)
    return fig, ax


exp1 = {"tree": ([0.81, 0.77, 0.79], [0.02, 0.03, 0.02]), "forest": ([0.86, 0.84, 0.85], [0.01, 0.02, 0.01])}
exp2 = {"lin": ([0.7, 0.72], [0.03, 0.02]), "mlp": ([0.78, 0.75], [0.04, 0.05]), "svm": ([0.76, 0.77], [0.02, 0.02])}
f1, a1 = compare_models(exp1, ["acc", "f1", "auc"], "experiment 1")
f2, a2 = compare_models(exp2, ["acc", "f1"], "experiment 2")
f1.savefig("exp1.png", dpi=50, bbox_inches="tight")
f2.savefig("exp2.png", dpi=50, bbox_inches="tight")
print(len(a1.containers), len(a2.containers), [t.get_text() for t in a2.get_legend().get_texts()])
