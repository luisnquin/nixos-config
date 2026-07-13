import fontforge
import sys


source, glyph_source, output = sys.argv[1:]
font = fontforge.open(source)
glyph = font.createChar(0xE9FF, "tailscale")
glyph.importOutlines(glyph_source)
glyph.width = font[ord("M")].width
glyph.correctDirection()
glyph.removeOverlap()
font.generate(output)
font.close()
