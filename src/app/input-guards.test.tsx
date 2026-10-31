import { fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { installInputGuards } from './input-guards'

let uninstall: (() => void) | null = null

function guard(blockBrowserKeys: boolean): void {
  uninstall = installInputGuards(window, { blockBrowserKeys })
}

function pressKey(key: string, ctrlKey = false, shiftKey = false): boolean {
  return fireEvent.keyDown(document.body, { key, ctrlKey, shiftKey })
}

afterEach(() => {
  uninstall?.()
  uninstall = null
})

describe('input guards', () => {
  it('suppresses the context menu outside text fields', () => {
    guard(false)
    render(
      <div>
        <p>Plain text</p>
        <input aria-label="Name" />
      </div>,
    )

    expect(fireEvent.contextMenu(screen.getByText('Plain text'))).toBe(false)
    expect(fireEvent.contextMenu(screen.getByRole('textbox', { name: 'Name' }))).toBe(true)
  })

  it('blocks reload, print and find keys in production builds', () => {
    guard(true)

    expect(pressKey('F5')).toBe(false)
    expect(pressKey('F3')).toBe(false)
    expect(pressKey('F7')).toBe(false)
    expect(pressKey('r', true)).toBe(false)
    expect(pressKey('R', true, true)).toBe(false)
    expect(pressKey('p', true)).toBe(false)
    expect(pressKey('f', true)).toBe(false)
  })

  it('blocks reload by its physical key on non-Latin layouts', () => {
    guard(true)

    expect(fireEvent.keyDown(document.body, { key: 'к', code: 'KeyR', ctrlKey: true })).toBe(false)
    expect(fireEvent.keyDown(document.body, { key: 'π', code: 'KeyP', ctrlKey: true })).toBe(false)
  })

  it('leaves ordinary typing and app shortcuts alone', () => {
    guard(true)

    expect(pressKey('r')).toBe(true)
    expect(pressKey('k', true)).toBe(true)
    expect(pressKey('0', true)).toBe(true)
  })

  it('does not block browser keys in development builds', () => {
    guard(false)

    expect(pressKey('F5')).toBe(true)
    expect(pressKey('r', true)).toBe(true)
  })
})
