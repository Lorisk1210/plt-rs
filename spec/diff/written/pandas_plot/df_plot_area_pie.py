# Area chart and pie chart straight from pandas (kind='area', Series.plot.pie with autopct).
import pandas as pd
import matplotlib.pyplot as plt

df = pd.DataFrame({"coal": [40, 35, 30, 25], "gas": [30, 32, 33, 30], "renewables": [10, 18, 25, 35]},
                  index=[2000, 2008, 2016, 2024])
fig, (a, b) = plt.subplots(1, 2, figsize=(10, 4))
df.plot.area(ax=a, alpha=0.7, title="generation mix")
df.loc[2024].plot.pie(ax=b, autopct="%.0f%%", ylabel="", title="2024")
print(len(a.collections), len(b.patches))
fig.savefig("mix.png")
current = plt.gcf()
print(len(current.axes), current.get_size_inches())
current.savefig("df_plot_area_pie_small.png", dpi=50)
