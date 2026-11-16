# pandas nullable Int64/Float64/boolean columns with missing values (pd.NA) passed to plot and bar.
import pandas as pd
import matplotlib.pyplot as plt

df = pd.DataFrame({"day": [1, 2, 3, 4, 5], "visits": pd.array([10, None, 14, 9, 12], dtype="Int64"),
                   "rate": pd.array([0.1, 0.2, None, 0.15, 0.3], dtype="Float64")})
fig, (a, b) = plt.subplots(1, 2, figsize=(8, 3))
a.plot(df["day"], df["visits"].astype("float"), "o-")
a.set_title("Int64 with NA -> float")
b.bar(df["day"], df["rate"].fillna(0))
b.set_title("Float64 filled")
print(df.dtypes.astype(str).to_dict())
print(a.lines[0].get_ydata().tolist())
fig.savefig("nullable.png")
