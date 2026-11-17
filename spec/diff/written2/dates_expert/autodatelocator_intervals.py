# AutoDateLocator's choices: intervald overrides, interval_multiples, minticks/maxticks per frequency, and the chosen rrule.
import datetime
import matplotlib.dates as mdates

start = datetime.datetime(2020, 2, 29, 13, 7, 3)
spans = [datetime.timedelta(seconds=7), datetime.timedelta(minutes=3), datetime.timedelta(hours=5), datetime.timedelta(days=3),
         datetime.timedelta(days=45), datetime.timedelta(days=800), datetime.timedelta(days=40000)]
for multiples in (True, False):
    loc = mdates.AutoDateLocator(interval_multiples=multiples)
    for span in spans:
        sub = loc.get_locator(start, start + span)
        ticks = loc.tick_values(start, start + span)
        print(multiples, span, type(sub).__name__, len(ticks), mdates.num2date(ticks[0]).strftime("%Y-%m-%d %H:%M:%S"))
loc = mdates.AutoDateLocator(minticks=2, maxticks={mdates.DAILY: 4, mdates.HOURLY: 12})
loc.intervald[mdates.DAILY] = [7]
print(len(loc.tick_values(start, start + datetime.timedelta(days=40))), len(loc.tick_values(start, start + datetime.timedelta(hours=30))))
print(sorted(loc.intervald.keys()), loc.intervald[mdates.HOURLY])
micro = mdates.AutoDateLocator()
ticks = micro.tick_values(start, start + datetime.timedelta(microseconds=500))
print(type(micro.get_locator(start, start + datetime.timedelta(microseconds=500))).__name__, len(ticks))
print(mdates.AutoDateLocator().nonsingular(19000.0, 19000.0))
try:
    mdates.AutoDateLocator().tick_values(start, start)
    print("same start/end ok")
except ValueError as e:
    print("ValueError", str(e)[:50])
for span in spans[:3]:
    loc = mdates.AutoDateLocator()
    ticks = loc.tick_values(start, start + span)
    print(span, [mdates.num2date(v).strftime("%H:%M:%S.%f") for v in ticks[:2]])
