import { act, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { renderWithProviders } from '../../test/render'
import { usePicturePreviewStore } from '../picture-preview-store'
import { PicturePreview } from './PicturePreview'

const PREVIEW = {
  owner: 'one',
  src: 'data:image/png;base64,AAAA',
  natural: { width: 192, height: 192 },
  anchor: { left: 100, right: 300, top: 100, bottom: 300 },
}

function show() {
  act(() => usePicturePreviewStore.getState().showPreview(PREVIEW))
}

function shown() {
  return screen.queryByTestId('picture-preview')
}

describe('PicturePreview', () => {
  beforeEach(() => {
    renderWithProviders(<PicturePreview />)
  })

  afterEach(() => {
    act(() => usePicturePreviewStore.getState().hidePreview())
  })

  it('shows nothing until a picture asks', () => {
    expect(shown()).not.toBeInTheDocument()
  })

  it('shows the picture beside its source', () => {
    show()
    const preview = shown()
    expect(preview).toBeInTheDocument()
    expect(preview).toHaveStyle({ left: '324px', width: '576px', height: '576px' })
    expect(preview?.querySelector('img')).toHaveAttribute('src', PREVIEW.src)
  })

  it.each([
    ['a key', () => new KeyboardEvent('keydown', { bubbles: true })],
    ['a press', () => new MouseEvent('pointerdown', { bubbles: true })],
    ['a wheel', () => new WheelEvent('wheel', { bubbles: true })],
    ['the window losing focus', () => new FocusEvent('blur')],
  ])('goes away on %s', (_name, event) => {
    show()
    act(() => {
      window.dispatchEvent(event())
    })
    expect(shown()).not.toBeInTheDocument()
  })

  it('goes away when the pointer leaves the window', () => {
    show()
    act(() => {
      document.body.dispatchEvent(
        new MouseEvent('mouseout', { bubbles: true, relatedTarget: null }),
      )
    })
    expect(shown()).not.toBeInTheDocument()
  })

  it('stays while the pointer moves from one element to another', () => {
    show()
    act(() => {
      document.body.dispatchEvent(
        new MouseEvent('mouseout', { bubbles: true, relatedTarget: document.documentElement }),
      )
    })
    expect(shown()).toBeInTheDocument()
  })
})
