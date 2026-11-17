# PDF Info dictionary: every standard key, datetime CreationDate/ModDate with time zones, Trapped values and bad keys.
import datetime
import io
import re
import warnings
import matplotlib
matplotlib.use("agg")
import matplotlib.pyplot as plt

fig, ax = plt.subplots(figsize=(2, 2))
ax.plot([0, 1])


def info(meta):
    buf = io.BytesIO()
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        try:
            fig.savefig(buf, format="pdf", metadata=meta)
        except Exception as e:
            return type(e).__name__, str(e)[:60]
    data = buf.getvalue()
    keys = re.findall(rb"/(Title|Author|Subject|Keywords|Creator|Producer|CreationDate|ModDate|Trapped) ", data)
    return sorted(k.decode() for k in keys), [str(w.message)[:60] for w in caught]


print(info({"CreationDate": None}))
print(info({"Title": "t", "Author": "a", "Subject": "s", "Keywords": "k", "Creator": "c", "Producer": "p",
            "CreationDate": None}))
tz = datetime.timezone(datetime.timedelta(hours=-5, minutes=-30))
for date in [datetime.datetime(2024, 1, 2, 3, 4, 5), datetime.datetime(2024, 1, 2, 3, 4, 5, tzinfo=datetime.timezone.utc),
             datetime.datetime(2024, 1, 2, 3, 4, 5, tzinfo=tz)]:
    buf = io.BytesIO()
    fig.savefig(buf, format="pdf", metadata={"CreationDate": date, "ModDate": date})
    print(re.findall(rb"/CreationDate \(([^)]*)\)", buf.getvalue()), re.findall(rb"/ModDate \(([^)]*)\)", buf.getvalue()))
for trapped in ("True", "False", "Unknown", True, "maybe"):
    print(trapped, info({"CreationDate": None, "Trapped": trapped}))
print(info({"CreationDate": None, "Colour": "red"}))
print(info({"CreationDate": None, "Title": 5}))
