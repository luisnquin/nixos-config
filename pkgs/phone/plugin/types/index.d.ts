export type Panel = { width: number; height: number }

export type Cells = { columns: number; rows: number }

export type View = {
  target: string | null
  panel: Panel | null
  cells: Cells | null
  error: string | null
  back: boolean
  grabbed: boolean
}

export type Element = { id: string; at: [number, number] }

export type Device = {
  id: string
  label: string
  host: string | null
  reach: string
  os: string
  kind: string
  hold: { project: string } | null
}

export type Input =
  | { kind: 'tap'; x: number; y: number }
  | { kind: 'swipe'; x: number; y: number; toX: number; toY: number }
  | { kind: 'key'; name: string }
  | { kind: 'type'; text: string }

declare module 'claude-code' {
  interface PluginState {
    phone: { view: View }
  }
}
