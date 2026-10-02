import { atom, read, update } from 'claude-code'
import type { ImageSource, Register } from 'claude-code'

import type { Cells, Device, Element, Input, Panel, View } from '../types'

const PANE = 'phone'
const KEY = 'screen'
const WIDTH = 432
const FRAME = 66
const TALLEST = 1024

const NAV = [
  ['back', 'back'],
  ['home', 'home'],
  ['app_switch', 'recents'],
] as const

const view = atom({ plugin: 'phone', key: 'view' } as const, {
  target: null,
  panel: null,
  cells: null,
  error: null,
  back: false,
  grabbed: false,
} as View)

const BLANK: ImageSource = { rgba: 'AAAA/w==', width: 1, height: 1 }

let loop = 0
let latest: ImageSource = BLANK
let asked = 0
let control: { dock: (columns: number) => void; resize: (cells: Cells) => void } | null = null
let running = ''
let grabbed = false

export function ready(devices: Device[]): string {
  const up = devices.filter(d => d.reach.startsWith('attached') || d.reach === 'online')
  if (!up.length) return 'nothing running; `phone boot <name>` first'
  return up.map(d => (d.host ? `${d.label} (${d.host})` : d.label)).join(', ')
}

const DESCRIPTION = 'Watch and drive a device beside the transcript; bare /phone picks a running device or closes the open one, /phone list shows them all'

export function fit(panel: Panel, rows: number): Cells {
  return { columns: Math.max(4, Math.round((2 * rows * panel.width) / panel.height)), rows }
}

export function within(panel: Panel, columns: number, rows: number): Cells {
  const tall = fit(panel, Math.max(4, Math.min(255, rows)))
  if (tall.columns <= columns) return tall
  return { columns: Math.max(4, columns), rows: Math.max(4, Math.floor((columns * panel.height) / (2 * panel.width))) }
}

export function pixels(panel: Panel): Panel {
  const even = (v: number) => Math.max(2, 2 * Math.round(v / 2))
  const width = Math.min(WIDTH, even((TALLEST * panel.width) / panel.height))
  return { width, height: Math.min(TALLEST, even((width * panel.height) / panel.width)) }
}

export function lastLine(buffer: string): { line: string | null; rest: string } {
  const end = buffer.lastIndexOf('\n')
  if (end < 0) return { line: null, rest: buffer }
  const start = buffer.lastIndexOf('\n', end - 1) + 1
  return { line: buffer.slice(start, end), rest: buffer.slice(end + 1) }
}

export function toDevice(panel: Panel, input: Input): string[] {
  const px = (v: number) => String(Math.round(v * panel.width))
  const py = (v: number) => String(Math.round(v * panel.height))
  switch (input.kind) {
    case 'tap':
      return ['tap', `${px(input.x)},${py(input.y)}`]
    case 'swipe':
      return ['swipe', `${px(input.x)},${py(input.y)}`, `${px(input.toX)},${py(input.toY)}`]
    case 'key':
      return ['key', input.name]
    case 'type':
      return ['type', input.text]
  }
}

export function choices(devices: Device[]): { label: string; target: string }[] {
  return devices
    .filter(d => d.reach.startsWith('attached') || d.reach === 'online')
    .slice(0, 4)
    .map(d => ({
      label: `${d.label} · ${d.os} ${d.kind}${d.host ? ` · ${d.host}` : ''}${d.hold ? ` · held by ${d.hold.project}` : ''}`,
      target: d.label,
    }))
}

export function backTap(elements: Element[]): string[] | null {
  const back = elements.find(e => e.id === 'BackButton')
  return back ? ['tap', `${back.at[0]},${back.at[1]}`] : null
}

export const register: Register = on => {
  on('session.start', async ($, e, next) => {
    $.ui.status(undefined)
    await $.command.register({ name: 'phone', description: DESCRIPTION, argumentHint: '[device] | stop' })
    void $.process.run(['phone', 'device', 'list', '--json']).then(list => {
      if (list.exitCode !== 0) return
      running = ready(JSON.parse(list.stdout) as Device[])
      $.ui.invalidate('command.describe')
    })

    return next(e)
  })

  on('command.describe', { command: 'phone' }, async ($, e, next) =>
    next(running ? { ...e, description: `${DESCRIPTION}. Running: ${running}` } : e),
  )

  on('command.run', { command: 'phone' }, async ($, e) => {
    const arg = e.args.trim()

    if (arg === 'list') {
      const list = await $.process.run(['phone', 'device', 'list'])
      return { text: (list.stdout || list.stderr).trimEnd() }
    }

    if (arg === 'stop' || (arg === '' && control)) {
      loop++
      control = null
      await $.ui.close({ id: PANE })
      return { text: `closed; /phone ${(await read($, view)).target} reopens it.` }
    }

    let chosen = arg
    if (!chosen) {
      const list = await $.process.run(['phone', 'device', 'list', '--json'])
      const options = list.exitCode === 0 ? choices(JSON.parse(list.stdout) as Device[]) : []
      if (!options.length) return { text: 'nothing running; `phone boot <name>` first, or /phone list' }
      const only = options.length === 1 ? options[0] : undefined
      const answer = only
        ? only.label
        : await $.ui.ask('Which device?', { header: 'phone', options: options.map(o => o.label) })
      chosen = options.find(o => o.label === answer)?.target ?? answer.trim()
    }
    const target = chosen

    const size = await $.process.run(['phone', 'size', '-t', target])
    const m = size.stdout.match(/(\d+)x(\d+)/)
    if (size.exitCode !== 0 || !m) {
      return { text: `${target} is not drivable: ${(size.stderr || size.stdout).trim()}` }
    }

    const list = await $.process.run(['phone', 'device', 'list', '--json'])
    const devices = list.exitCode === 0 ? (JSON.parse(list.stdout) as Device[]) : []
    const back = devices.find(d => d.label === target || d.id === target)?.os !== 'ios'

    const panel = { width: Number(m[1]), height: Number(m[2]) }
    const { width, height } = pixels(panel)

    let shown: Cells | null = null

    const watch = async () => {
      const token = ++loop
      latest = BLANK
      const name = `pane-${Math.round(await $.clock.now())}-${token}`
      const argv = ['phone', 'stream', '-t', target, '--size', `${width}x${height}`, '--shm', name]
      let buffer = ''
      let error = ''
      let due = false

      const flush = (tries = 0) => {
        if (due) return
        due = true
        void $.clock.sleep(FRAME).then(async () => {
          due = false
          if (token !== loop || !shown) return
          const sent = await $.ui.blit({ requestId: PANE, key: KEY, source: latest, columns: shown.columns, rows: shown.rows })
          if (sent.deny && tries < 15) flush(tries + 1)
        })
      }

      for await (const { stream, text } of $.process.spawn({ argv })) {
        if (token !== loop) break
        if (stream === 'stderr') {
          error += text
          continue
        }

        const { line, rest } = lastLine(buffer + text)
        buffer = rest
        if (!line) continue

        latest = { file: `/dev/shm${line}`, format: 'rgb', width, height }
        flush()
      }

      if (token !== loop) return
      const reason = error.trim().split('\n').pop() || 'the stream ended'
      await update($, view, v => ({ ...v, error: reason }))
    }

    const open = (columns?: number) =>
      $.ui.open({ id: PANE, title: target, holdToasts: true, ...(columns ? { columns } : {}) })

    control = {
      dock: columns => void open(columns),
      resize: cells => {
        const first = !shown
        shown = cells
        void update($, view, v => ({ ...v, cells }))
        if (first) void watch()
      },
    }
    asked = 0
    grabbed = false
    await update($, view, () => ({ target, panel, cells: null, error: null, back, grabbed }))
    await open()

    return { text: `streaming ${target}.` }
  })

  on('ui.message', async ($, e, next) => {
    if (e.requestId !== PANE) return next(e)
    const v = await read($, view)
    if (!v.target || !v.panel) return {}
    if (!grabbed) {
      grabbed = true
      void update($, view, s => ({ ...s, grabbed }))
    }
    void $.process.run(['phone', '-t', v.target, ...toDevice(v.panel, e.data as Input)])
    return {}
  })

  on('prompt.edit', async ($, e, next) => {
    if (grabbed) {
      grabbed = false
      void update($, view, s => ({ ...s, grabbed }))
    }
    return next(e)
  })

  on('ui.close', async ($, e, next) => {
    if (e.id === PANE) {
      loop++
      control = null
    }
    return next(e)
  })

  on('ui.render', { component: 'Pane', requestId: PANE }, async ($, e, next) => {
    if (e.surface !== 'terminal') return next(e)
    const { Box, Text, Image, Client, Button } = $.ui.resolve(e)
    const v = await read($, view)
    if (!v.panel) return <Text dimColor>/phone &lt;target&gt;</Text>
    if (v.error) return <Text color="red">{`${v.error}\n\n/phone ${v.target} restarts it`}</Text>
    if (!control) return <Text dimColor>{`stream stopped on reload; /phone ${v.target} restarts it`}</Text>

    const rows = e.props.scroll.bodyRows - 1
    const tall = fit(v.panel, Math.min(255, rows))
    if (tall.columns !== e.props.bodyColumns && asked !== tall.columns) {
      asked = tall.columns
      control?.dock(tall.columns)
    }

    const want = within(v.panel, e.props.bodyColumns, rows)
    if (!v.cells || want.columns !== v.cells.columns || want.rows !== v.cells.rows) control?.resize(want)
    if (!v.cells) return <Text dimColor>starting…</Text>

    const { columns, rows: high } = v.cells
    const target = v.target
    if (!target) return <Text dimColor>/phone &lt;target&gt;</Text>

    const press = async (name: string) => {
      if (name !== 'back' || v.back) return $.process.run(['phone', '-t', target, 'key', name])
      const shot = await $.process.run(['phone', '-t', target, 'snapshot', '--json'])
      const tap = shot.exitCode === 0 ? backTap(JSON.parse(shot.stdout) as Element[]) : null
      if (tap) return $.process.run(['phone', '-t', target, ...tap])
    }

    return (
      <Box flexDirection="column">
        <Box position="relative" width={columns} height={high}>
          <Image key={KEY} source={latest} columns={columns} rows={high} alt="no kitty graphics here" />
          <Box position="absolute">
            <Client key="input" module="./overlay.tsx" width={columns} height={high} props={v.cells} />
          </Box>
        </Box>
        <Box
          flexDirection="row"
          gap={3}
          width={columns}
          {...(e.props.isFocused || v.grabbed ? { backgroundColor: 'green' } : {})}
        >
          {NAV.map(([name, label]) => (
            <Button key={`nav-${name}`} onPress={() => void press(name)}>
              {label}
            </Button>
          ))}
          {(e.props.isFocused || v.grabbed) && (
            <Text color="black" bold wrap="truncate-end">
              esc
            </Text>
          )}
        </Box>
      </Box>
    )
  })
}
