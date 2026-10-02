import type { ClientKeyEvent, ClientModule } from 'claude-code'

import type { Input } from '../types'

type Size = { columns: number; rows: number }
type Local = { ready: true }

let latest: Size = { columns: 1, rows: 1 }

const KEYS: Record<string, string> = {
  return: 'enter',
  backspace: 'del',
  delete: 'forward_del',
  tab: 'tab',
  up: 'dpad_up',
  down: 'dpad_down',
  left: 'dpad_left',
  right: 'dpad_right',
  home: 'home',
  pageup: 'page_up',
  pagedown: 'page_down',
}

export function keyInput(k: ClientKeyEvent): Input | null {
  if (k.ctrl && k.key === 'b') return { kind: 'key', name: 'back' }
  if (k.ctrl && k.key === 'h') return { kind: 'key', name: 'home' }
  if (k.ctrl && k.key === 'r') return { kind: 'key', name: 'app_switch' }
  if (k.ctrl || k.meta) return null
  const name = KEYS[k.key]
  if (name) return { kind: 'key', name }
  return [...k.key].length === 1 ? { kind: 'type', text: k.key } : null
}

const Overlay: ClientModule<Size, Local> = (box, surface) => {
  const { Box } = surface.elements
  latest = box

  if (surface.state === undefined) {
    let down: { x: number; y: number } | null = null

    surface.onPointer(p => {
      const pos = p.fine ?? { x: p.x + 0.5, y: p.y + 0.5 }
      const x = pos.x / latest.columns
      const y = pos.y / latest.rows
      if (p.type === 'down' && p.button === 'left') down = { x, y }
      if (p.type === 'up' && down) {
        const moved = Math.hypot(x - down.x, (y - down.y) / 2) > 0.02
        surface.post(
          moved
            ? { kind: 'swipe', x: down.x, y: down.y, toX: clamp(x), toY: clamp(y) }
            : { kind: 'tap', x: down.x, y: down.y },
        )
        down = null
      }
    })
    surface.onKey(k => {
      const input = keyInput(k)
      if (input) surface.post(input)
    })
    surface.setState({ ready: true })
  }

  return <Box width={box.columns} height={box.rows} />
}

function clamp(v: number) {
  return Math.min(1, Math.max(0, v))
}

export default Overlay
