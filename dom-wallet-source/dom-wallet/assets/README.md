# Assets

Optional binary assets the build picks up if present.

| File              | Used by      | Effect when missing                                     |
| ----------------- | ------------ | ------------------------------------------------------- |
| `dom.ico`         | `build.rs`   | The executable ships without an embedded Windows icon   |
| `dom-coin.png`    | reserved     | (Not currently used — `ui/hero.rs` draws the coin from primitives) |

`dom.ico` should be a multi-resolution Windows icon (16, 32, 48, 256 px) of the DOM coin glyph rendered in amber on transparent or dark background. Generate from the SVG master with ImageMagick:

```bash
convert -background none dom-coin.svg \
    \( -clone 0 -resize 16x16 \) \
    \( -clone 0 -resize 32x32 \) \
    \( -clone 0 -resize 48x48 \) \
    \( -clone 0 -resize 256x256 \) \
    -delete 0 dom.ico
```

These files are intentionally not committed — the build is fully functional without them, and the visual identity is owned outside this repository.
