import { act, fireEvent, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { TEST_SETTINGS } from '../../test/fixtures'
import { installIpcMock } from '../../test/ipc-mock'
import { renderWithProviders } from '../../test/render'
import { usePicturePreviewStore } from '../picture-preview-store'
import { ItemPicture } from './ItemPicture'

const PICTURE = 'data:image/png;base64,AAAA'

function renderPicture(id: string | null, hidePictures = false, hoverPreview = true) {
  const ipc = installIpcMock(undefined, { handlers: { assets_picture: () => PICTURE } })
  const settings = {
    ...TEST_SETTINGS,
    appearance: { ...TEST_SETTINGS.appearance, hidePictures, hoverPreview },
  }
  return {
    ipc,
    ...renderWithProviders(<ItemPicture id={id} label="Blouse" width={40} height={40} />, {
      settings,
    }),
  }
}

describe('ItemPicture', () => {
  it('shows the stored picture', async () => {
    renderPicture('192-a')
    expect(await screen.findByAltText('Blouse')).toHaveAttribute('src', PICTURE)
  })

  it('shows an empty tile without a picture', async () => {
    const { ipc } = renderPicture(null)
    expect(screen.getByRole('img', { name: 'Blouse' })).toBeVisible()
    await waitFor(() => expect(ipc.callsOf('assets_picture')).toEqual([]))
  })

  it('loads nothing while pictures are hidden', async () => {
    const { ipc } = renderPicture('192-a', true)
    expect(screen.getByRole('img', { name: 'Blouse' })).toBeVisible()
    await waitFor(() => expect(ipc.callsOf('assets_picture')).toEqual([]))
    expect(screen.queryByAltText('Blouse')).not.toBeInTheDocument()
  })
})

describe('the big picture on hover', () => {
  const preview = () => usePicturePreviewStore.getState().preview
  let restore: () => void

  async function tileOf(hoverPreview = true) {
    const rendered = renderPicture('192-a', false, hoverPreview)
    const image = await screen.findByAltText('Blouse')
    const tile = image.parentElement
    if (!tile) throw new Error('no tile')
    vi.useFakeTimers()
    return { ...rendered, tile }
  }

  function enter(tile: HTMLElement, pointerType = 'mouse') {
    fireEvent.pointerEnter(tile, { pointerType })
  }

  function wait(ms: number) {
    act(() => {
      vi.advanceTimersByTime(ms)
    })
  }

  beforeEach(() => {
    const natural = vi.spyOn(HTMLImageElement.prototype, 'naturalWidth', 'get').mockReturnValue(192)
    const height = vi.spyOn(HTMLImageElement.prototype, 'naturalHeight', 'get').mockReturnValue(192)
    restore = () => {
      natural.mockRestore()
      height.mockRestore()
    }
  })

  afterEach(() => {
    vi.useRealTimers()
    restore()
    usePicturePreviewStore.setState({ preview: null })
  })

  it('opens after half a second, not before', async () => {
    const { tile } = await tileOf()

    enter(tile)
    wait(499)
    expect(preview()).toBeNull()
    wait(1)

    expect(preview()).toMatchObject({ src: PICTURE, natural: { width: 192, height: 192 } })
  })

  it('never opens when the pointer leaves in time', async () => {
    const { tile } = await tileOf()

    enter(tile)
    wait(300)
    fireEvent.pointerLeave(tile)
    wait(1000)

    expect(preview()).toBeNull()
  })

  it('closes when the pointer leaves', async () => {
    const { tile } = await tileOf()

    enter(tile)
    wait(500)
    fireEvent.pointerLeave(tile)

    expect(preview()).toBeNull()
  })

  it.each([
    ['press', fireEvent.pointerDown],
    ['wheel', fireEvent.wheel],
  ])('gives up waiting on a %s', async (_name, interrupt) => {
    const { tile } = await tileOf()

    enter(tile)
    wait(300)
    interrupt(tile)
    wait(1000)

    expect(preview()).toBeNull()
  })

  it('stays off when the setting is off', async () => {
    const { tile } = await tileOf(false)

    enter(tile)
    wait(1000)

    expect(preview()).toBeNull()
  })

  it('ignores touch, which has no hover', async () => {
    const { tile } = await tileOf()

    enter(tile, 'touch')
    wait(1000)

    expect(preview()).toBeNull()
  })

  it('stops waiting and closes when a virtual list drops the picture', async () => {
    const { tile, unmount } = await tileOf()

    enter(tile)
    wait(300)
    unmount()
    wait(1000)
    expect(preview()).toBeNull()
  })

  it('leaves a preview of another picture alone when it goes away', async () => {
    const { tile, unmount } = await tileOf()
    const other = { ...previewOf(tile), owner: 'other' }
    act(() => usePicturePreviewStore.getState().showPreview(other))

    unmount()

    expect(preview()).toEqual(other)
  })
})

function previewOf(tile: HTMLElement) {
  const { left, right, top, bottom } = tile.getBoundingClientRect()
  return {
    owner: 'this',
    src: PICTURE,
    natural: { width: 192, height: 192 },
    anchor: { left, right, top, bottom },
  }
}
