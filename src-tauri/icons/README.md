# App icons

`icon.svg` is the source (Quiet Canvas violet rounded square with a stacked-layers glyph). Every other
file here is generated from it. Run both commands from this folder (needs `rsvg-convert` and ImageMagick 7):

```sh
rsvg-convert -w 1024 -h 1024 -o "${TMPDIR:-/tmp}/remmods-icon-1024.png" icon.svg
magick "${TMPDIR:-/tmp}/remmods-icon-1024.png" \( +clone -resize 32x32 -write PNG32:32x32.png +delete \) \( +clone -resize 128x128 -write PNG32:128x128.png +delete \) \( +clone -resize 256x256 -write PNG32:128x128@2x.png +delete \) \( +clone -resize 512x512 -write PNG32:icon.png +delete \) -define icon:auto-resize=256,128,64,48,32,24,16 icon.ico
```

The PNGs must stay 8-bit RGBA (`PNG32:`): `tauri::generate_context!` embeds `32x32.png` as the Linux window
icon and rejects other colour types. `tauri-build` embeds `icon.ico` into the Windows executable.
