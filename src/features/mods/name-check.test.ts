import { describe, expect, it } from 'vitest'
import { makeModRow } from '../../test/fixtures'
import { checkName, foldName, siblingFolds } from './name-check'

const NONE: ReadonlySet<string> = new Set()
const TEMP = `_x_tmp_${'a'.repeat(32)}`

describe('checkName', () => {
  it.each([
    ['', 'empty'],
    ['.', 'dot_name'],
    ['..', 'dot_name'],
    ['a'.repeat(256), 'too_long'],
    ['é'.repeat(128), 'too_long'],
    ['a<b', 'forbidden_char'],
    ['a:b', 'forbidden_char'],
    ['a/b', 'forbidden_char'],
    ['a\\b', 'forbidden_char'],
    ['a|b', 'forbidden_char'],
    ['a?b', 'forbidden_char'],
    ['a*b', 'forbidden_char'],
    ['a"b', 'forbidden_char'],
    ['a\tb', 'control_char'],
    ['a‮b', 'bidi_control'],
    ['a‏b', 'bidi_control'],
    ['Skin.', 'trailing_dot_or_space'],
    ['Skin ', 'trailing_dot_or_space'],
    ['CON', 'reserved_device_name'],
    ['nul.txt', 'reserved_device_name'],
    ['COM1', 'reserved_device_name'],
    ['lpt²', 'reserved_device_name'],
    ['CONIN$', 'reserved_device_name'],
    ['.disabled_Skin', 'disabled_prefix'],
    ['.DISABLED_Skin', 'disabled_prefix'],
    [TEMP, 'temp_name'],
  ])('refuses %j as %s', (name, code) => {
    expect(checkName(name, 'Old', NONE)).toBe(code)
  })

  it('accepts ordinary names', () => {
    for (const name of ['Cool Skin', 'COM0', 'COM10', '日本', 'a.b', '_tmp_short', 'CONSOLE']) {
      expect(checkName(name, 'Old', NONE), name).toBeNull()
    }
  })

  it('reports an unchanged name before any other rule', () => {
    expect(checkName('Same', 'Same', NONE)).toBe('unchanged')
  })

  it('reports a sibling that differs only in case or normalisation', () => {
    const siblings = new Set([foldName('Cool Skin'), foldName('Café')])
    expect(checkName('cool skin', 'Old', siblings)).toBe('exists')
    expect(checkName('Cafe', 'Old', siblings)).toBeNull()
    expect(checkName('Café', 'Old', siblings)).toBe('exists')
  })
})

describe('siblingFolds', () => {
  it('collects the other rows of the same root, both forms for LogicMods', () => {
    const self = makeModRow('Self')
    const folds = siblingFolds(
      [
        self,
        makeModRow('Other'),
        makeModRow('Elsewhere', { root: 'Logic' }),
        makeModRow('Gate', { kind: 'logic', root: 'Mods' }),
      ],
      self,
    )
    expect(folds).toEqual(new Set(['other', 'gate', '.disabled_gate']))
  })
})
