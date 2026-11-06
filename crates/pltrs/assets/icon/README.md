# pltrs's application icon

`pltrs.svg` is the design source of the icon pltrs's windows show ("E:
Contour", the user's choice among five variants a designer drew in Paper,
2026-11-06): filled contour bands in viridis colours rising to a lime peak,
on a deep violet gradient squircle (824 units wide on Apple's 1024-unit
icon grid) with a soft drop shadow.

pltrs draws the icon itself, from this geometry, at each size a platform
asks for: `crates/pltrs/src/gui/app_icon.rs` holds the design as data
(its tests check it against this file) and a bolder rendition for 32
pixels and less.
