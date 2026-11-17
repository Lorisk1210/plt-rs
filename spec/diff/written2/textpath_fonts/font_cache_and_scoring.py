# Score fonts as findfont does for a list of properties and compare with the chosen file; cache hits through _findfont_cached.
import os
from matplotlib import font_manager as fm

mgr = fm.fontManager
props = [
    fm.FontProperties(family=["DejaVu Sans"], weight="bold", style="italic"),
    fm.FontProperties(family=["DejaVu Serif"], stretch="condensed"),
    fm.FontProperties(family=["serif"], weight=300),
    fm.FontProperties(family=["STIXGeneral"], weight="bold"),
    fm.FontProperties(family=["no such family"]),
]
for fp in props:
    best = None
    for entry in mgr.ttflist:
        score = (mgr.score_family(fp.get_family(), entry.name) * 10 + mgr.score_style(fp.get_style(), entry.style)
                 + mgr.score_variant(fp.get_variant(), entry.variant) + mgr.score_weight(fp.get_weight(), entry.weight)
                 + mgr.score_stretch(fp.get_stretch(), entry.stretch) + mgr.score_size(fp.get_size(), entry.size))
        if best is None or score < best[0]:
            best = (score, entry.fname)
    chosen = fm.findfont(fp, fallback_to_default=True)
    print(fp.get_family(), round(best[0], 3), os.path.basename(best[1]), os.path.basename(chosen))
info_before = mgr._findfont_cached.cache_info()
for _ in range(3):
    fm.findfont(props[0])
info_after = mgr._findfont_cached.cache_info()
print(info_after.hits - info_before.hits, info_after.misses - info_before.misses)
print(mgr.defaultFamily, mgr.default_size)
print(sorted(mgr.defaultFont), os.path.basename(mgr.defaultFont["ttf"]))
print(fm.is_opentype_cff_font(fm.findfont("DejaVu Sans")))
print(sorted(fm.get_fontext_synonyms("ttf")), sorted(fm.get_fontext_synonyms("afm")))
print(len(fm.list_fonts(os.path.dirname(fm.findfont("DejaVu Sans")), ["ttf"])) > 30)
