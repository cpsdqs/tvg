# tvg
Decoder for the toon boom harmony drawing format.
The tvg format is unfortunately not documented, so this is the result of a whole lot of guessing from looking at the files in rehex and the tvg2xml output.

Can currently decode:

- Misc. file metadata
- Color palettes
- Layer data:
    - Shape colors
    - Fill shapes (shapes created using the brush tool or the fill bucket)
    - Stroke center lines (thickness remains a mystery)
