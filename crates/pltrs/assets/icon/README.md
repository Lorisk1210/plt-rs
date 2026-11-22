# The pltrs application icon

`pltrs.svg` is the design source of the icon on pltrs windows. Filled
viridis contour bands rise to a lime peak on a deep violet gradient
squircle with a soft drop shadow. The squircle is 824 units wide on the
1024-unit Apple icon grid.

pltrs draws the icon from this geometry at each size that a platform
asks for. `crates/pltrs/src/gui/app_icon.rs` holds the design as data,
and its tests check the data against `pltrs.svg`. It also holds a bolder
version for 32 pixels and less.
