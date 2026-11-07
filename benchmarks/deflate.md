# DEFLATE: zlib-ng's parse (2026-11-07)

pltrs's DEFLATE encoder (`crates/pltrs-codec/src/deflate/encode`) is a
port of zlib-ng 2.3's: for the same input, level and strategy it writes
zlib-ng's bytes, checked against a zlib-ng 2.3.3 build
(`crates/pltrs-codec/tools/zlib_ng`). PNG images are filtered by
Pillow's heuristic and compressed as Pillow feeds zlib-ng: a row at a
time, into 64 KiB buffers, one `IDAT` chunk each. Every PNG, PDF and
SVG-embedded image changed its compressed bytes once with it; pixels and
document structure did not. [`README.md`](README.md) predates the change;
the next `cargo xtask bench --report` includes it.

## DEFLATE alone

Level 6, one stream at a time, on one core of MS-02 (AMD Ryzen 9 9955HX),
median of 31 runs:

| input | before | after | zlib-ng C, SIMD / generic |
|---|---:|---:|---:|
| 4.0 MB of PNG rows (`mpl_image/imshow_antialiased_double`) | 52.7 ms | 23.6 ms | 21.8 / 24.3 ms |
| 2.9 MB of PNG rows (`gallery_misc/topographic_hillshading`) | 41.8 ms | 28.7 ms | 27.8 / 29.7 ms |
| 3.4 MB of PNG rows (`gallery_images/plot_streamplot_1`) | 20.7 ms | 15.9 ms | 14.9 / 16.9 ms |
| 1.2 MB of PDF path text | 35.3 ms | 21.6 ms | |

PNG encoding with filtering, same machine: 54.8 → 25.0 ms on the 4.0 MB
image, 43.5 → 30.0 ms and 22.9 → 17.5 ms on the others. Level 9 on the
first image: 770–920 → 113 ms.

## The PNG and PDF workloads

`cargo xtask bench meshes/imshow medium/imshow meshes/pcolormesh
output/lissajous_1e5_pdf lines/plot_1e6_png lines/plot_1e6_pdf small/
--against` the pack of `b414751d`, this build's median over the other's:

| | MS-02 | Framework laptop (Ryzen AI 9 HX 370) |
|---|---:|---:|
| pltrs (Rust) warm, typically | −15.2% (20 faster, 1 within noise, 2 slower) | −15.2% (20 / 0 / 3) |
| pltrs (Python) warm, typically | −12.7% (22 / 1 / 0) | −13.2% (22 / 1 / 0) |
| pltrs (Rust) cold, typically | −14.2% | −9.5% |
| pltrs (Python) cold, typically | −0.8% | −3.0% |
| `output/lissajous_1e5_pdf`, Rust / Python warm | −38.5% / −37.7% | −39.1% / −38.3% |
| `small/*_png`, Rust warm | −15% to −36% | −16% to −35% |
| `meshes/imshow_60_*`, Rust warm | −11% to −13% | −8% to −12% |

The laptop's column is of the build before PDF streams shared one
compressor's tables; there `small/line_pdf` was 23% slower in
pltrs (Rust) (zlib-ng allocates and clears 300 KiB of tables per stream,
and a small PDF has some thirty streams), now 10.8% faster on MS-02.

Two workloads are slower in pltrs (Rust) and faster in pltrs (Python) on
both machines: `meshes/imshow_4000_antialiased` (+5.3% / −8.4% on MS-02)
and `meshes/imshow_2000_bicubic_colorbar` (+9.7% / −10.0%). Their PNG
encoding is faster in either build (13.3 → 10.8 ms on the image they
write) and the page faults are as many; stack samples put the difference
in unchanged code (resampling, normalisation, colormap lookup), most
likely how that code is inlined and placed in the two benchmark
binaries rather than compression.
