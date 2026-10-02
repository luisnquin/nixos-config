import { expect, test } from 'claude-code/testing'

import { backTap, choices, fit, lastLine, pixels, ready, toDevice, within } from '../hooks/register'
import { keyInput } from '../hooks/overlay'

const panel = { width: 1080, height: 2400 }

test('fit sizes the columns from the panel aspect, two pixel rows to a cell', async () => {
  expect(fit(panel, 32)).toEqual({ columns: 29, rows: 32 })
})

test('pixels keep the panel aspect within the stream limits', async () => {
  expect(pixels(panel)).toEqual({ width: 432, height: 960 })
  expect(pixels({ width: 1080, height: 2640 })).toEqual({ width: 418, height: 1022 })
})

test('a narrow dock shrinks the view by width', async () => {
  expect(within(panel, 100, 52)).toEqual({ columns: 47, rows: 52 })
  expect(within(panel, 29, 52)).toEqual({ columns: 29, rows: 32 })
})

test('only the newest whole frame is drawn', async () => {
  expect(lastLine('AA')).toEqual({ line: null, rest: 'AA' })
  expect(lastLine('AA\nBB\nC')).toEqual({ line: 'BB', rest: 'C' })
  expect(lastLine('AA\n')).toEqual({ line: 'AA', rest: '' })
})

test('pointer fractions land in panel pixels', async () => {
  expect(toDevice(panel, { kind: 'tap', x: 0.5, y: 0.25 })).toEqual(['tap', '540,600'])
  expect(toDevice(panel, { kind: 'swipe', x: 0.5, y: 0.8, toX: 0.5, toY: 0.2 })).toEqual([
    'swipe',
    '540,1920',
    '540,480',
  ])
})

test('keys map to phone verbs', async () => {
  expect(keyInput({ key: 'return' })).toEqual({ kind: 'key', name: 'enter' })
  expect(keyInput({ key: 'b', ctrl: true })).toEqual({ kind: 'key', name: 'back' })
  expect(keyInput({ key: 'x' })).toEqual({ kind: 'type', text: 'x' })
  expect(keyInput({ key: 'c', ctrl: true })).toBeNull()
})

test('choices offers running devices with what they are and where', () => {
  expect(
    choices([
      { id: 'sim:a', label: 'iPhone 17 Pro', os: 'ios', kind: 'sim', host: 'rose', reach: 'online', hold: null },
      { id: 'f', label: 'faraday', os: 'android', kind: 'phys', host: null, reach: 'attached/tcp', hold: { project: 'dazzle' } },
      { id: 'avd:b', label: 'pixel_7-api36-b', os: 'android', kind: 'emu', host: 'rose', reach: 'off', hold: null },
    ]),
  ).toEqual([
    { label: 'iPhone 17 Pro · ios sim · rose', target: 'iPhone 17 Pro' },
    { label: 'faraday · android phys · held by dazzle', target: 'faraday' },
  ])
})

test('ready lists running devices with their host', async () => {
  expect(
    ready([
      { id: 'avd:flexa', label: 'flexa', os: 'android', kind: 'emu', hold: null, host: null, reach: 'attached/emu' },
      { id: 'sim:a', label: 'iPhone 17 Pro', os: 'ios', kind: 'sim', hold: null, host: 'rose', reach: 'online' },
      { id: 'avd:b', label: 'pixel_7-api36-b', os: 'android', kind: 'emu', hold: null, host: 'rose', reach: 'off' },
    ]),
  ).toBe('flexa, iPhone 17 Pro (rose)')
  expect(ready([])).toContain('phone boot')
})

test('backTap taps the navigation bar back button', () => {
  expect(backTap([{ id: '', at: [201, 437] }, { id: 'BackButton', at: [38, 84] }])).toEqual(['tap', '38,84'])
  expect(backTap([{ id: '', at: [201, 437] }])).toBeNull()
})
