# App icons

The source is `icon.png` in the repository root: the owner's illustration (a ninja holding a fox mask),
1024×1024 with a transparent background, cut to the character and set on the bottom edge as a bust.
Every file here is generated from it. Run this from this folder (needs ImageMagick 7):

```sh
magick ../../icon.png -filter Lanczos \( +clone -resize 32x32 -write PNG32:32x32.png +delete \) \( +clone -resize 128x128 -write PNG32:128x128.png +delete \) \( +clone -resize 256x256 -write PNG32:128x128@2x.png +delete \) \( +clone -resize 512x512 -write PNG32:icon.png +delete \) -define icon:auto-resize=256,128,64,48,32,24,16 icon.ico
```

The PNGs must stay 8-bit RGBA (`PNG32:`): `tauri::generate_context!` embeds `32x32.png` as the Linux window
icon and rejects other colour types. `tauri-build` embeds `icon.ico` into the Windows executable.
`cachyos-install.sh` installs the four PNGs into the hicolor theme (32, 128, 256 and 512 px).
