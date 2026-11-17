# font_manager queries: findfont for generic families, weights and stretches, fallback lists, json round trip of a FontManager.
import os
import tempfile
import matplotlib
from matplotlib import font_manager as fm

fm_ = fm.fontManager
queries = [
    "sans-serif", "serif", "monospace", "DejaVu Sans:bold", "DejaVu Serif:italic",
    "STIXGeneral", "cmr10", "DejaVu Sans:weight=light", "DejaVu Sans Mono:style=oblique",
]
for q in queries:
    fp = fm.FontProperties(family=[q]) if ":" not in q else fm.FontProperties(q)
    print(q, "->", os.path.basename(fm.findfont(fp, fallback_to_default=True)))
fp = fm.FontProperties(family=["DejaVu Sans", "STIXGeneral"], weight=700, style="italic", size="large")
print(fp.get_family(), fp.get_weight(), fp.get_style(), fp.get_size_in_points())
print(os.path.basename(fm.findfont(fp)))
print([os.path.basename(f) for f in fm_._find_fonts_by_props(fm.FontProperties(family=["DejaVu Sans", "STIXGeneral"]))])
print(fm.weight_dict["bold"], fm.weight_dict["light"], sorted(k for k in fm.font_scalings if k)[:3])
fp2 = fm.FontProperties("DejaVu Sans:size=9")
print(fp2.get_size(), fp2 == fm.FontProperties(family="DejaVu Sans", size=9), hash(fp2) == hash(fp2.copy()))
dejavu = [f for f in fm_.ttflist if f.name == "DejaVu Sans"]
print(len(dejavu) >= 4, sorted({f.weight for f in dejavu})[:3], sorted({f.style for f in dejavu}))
print(fm_.score_family("sans-serif", "DejaVu Sans"), fm_.score_weight(400, 700), fm_.score_style("italic", "oblique"))
print(fm_.score_size(12, 12), fm_.score_stretch("normal", "condensed"))
print(fm.get_font_names().count("DejaVu Sans"))
with tempfile.TemporaryDirectory(dir=".") as d:
    fname = os.path.join(d, "fm.json")
    fm.json_dump(fm_, fname)
    loaded = fm.json_load(fname)
    print(type(loaded).__name__, len(loaded.ttflist) == len(fm_.ttflist))
