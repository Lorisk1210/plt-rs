# FontProperties: every setter, fontconfig pattern round trips, copies, equality and the size names.
from matplotlib.font_manager import FontProperties
from matplotlib import rcParams

fp = FontProperties()
print(fp.get_family(), fp.get_style(), fp.get_variant(), fp.get_weight(), fp.get_stretch(), fp.get_size())
fp.set_family(["Liberation Sans", "DejaVu Sans"])
fp.set_style("oblique")
fp.set_variant("small-caps")
fp.set_weight("semibold")
fp.set_stretch("condensed")
fp.set_size("x-small")
pattern = fp.get_fontconfig_pattern()
print(pattern)
back = FontProperties(pattern)
print(back == fp, back.get_family(), back.get_weight(), back.get_stretch(), back.get_size_in_points())
for size in ("xx-small", "x-small", "small", "medium", "large", "x-large", "xx-large", "larger", "smaller", 7, 13.5):
    p = FontProperties(size=size)
    print(size, p.get_size_in_points())
for weight in (100, 250, "ultralight", "heavy", 900):
    print(weight, FontProperties(weight=weight).get_weight())
for bad in [dict(weight="enormous"), dict(size="gigantic"), dict(stretch=1200), dict(style="slanted")]:
    try:
        FontProperties(**bad)
    except ValueError as e:
        print("ValueError", str(e)[:60])
c = fp.copy()
c.set_size(20)
print(fp.get_size(), c.get_size(), fp == c, str(fp) == fp.get_fontconfig_pattern())
rcParams["font.size"] = 14
print(FontProperties(size="large").get_size_in_points(), FontProperties().get_size())
print(FontProperties(family="monospace", math_fontfamily="stix").get_math_fontfamily())
print(sorted(FontProperties.__dict__)[:3] != [], fp.get_slant(), fp.get_name() != "")
