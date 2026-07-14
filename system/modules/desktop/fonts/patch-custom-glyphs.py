import fontforge
import sys


source, tailscale_source, memory_source, output = sys.argv[1:]
font = fontforge.open(source)

for codepoint, name, glyph_source in (
    (0xE9FF, "tailscale", tailscale_source),
    (0xE9FE, "memory_module", memory_source),
):
    glyph = font.createChar(codepoint, name)
    glyph.importOutlines(glyph_source)
    glyph.width = font[ord("M")].width
    glyph.correctDirection()
    glyph.removeOverlap()

font.generate(output)
font.close()
