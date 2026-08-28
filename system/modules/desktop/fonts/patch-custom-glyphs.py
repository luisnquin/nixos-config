import fontforge
import sys


(
    source,
    tailscale_source,
    memory_source,
    ssh_outbound_source,
    ssh_inbound_source,
    claude_source,
    codex_source,
    output,
) = sys.argv[1:]
font = fontforge.open(source)

cell_width = font[ord("M")].width
compact_arrow_width = 830

for codepoint, name, glyph_source, autoinstruct, width in (
    (0xE9FF, "tailscale", tailscale_source, False, cell_width),
    (0xE9FE, "memory_module", memory_source, False, cell_width),
    (0xE9FC, "ssh_outbound", ssh_outbound_source, True, compact_arrow_width),
    (0xE9FD, "ssh_inbound", ssh_inbound_source, True, compact_arrow_width),
    (0xE9FB, "agent_claude", claude_source, False, cell_width),
    (0xE9FA, "agent_codex", codex_source, False, cell_width),
):
    glyph = font.createChar(codepoint, name)
    glyph.importOutlines(glyph_source)
    glyph.width = width
    glyph.correctDirection()
    glyph.removeOverlap()
    if autoinstruct:
        glyph.simplify()
        glyph.round()
        glyph.autoHint()
        glyph.autoInstr()

font.generate(output)
font.close()
