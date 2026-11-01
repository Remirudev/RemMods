import { vi } from 'vitest'

// jsdom does no layout, so the virtualizer would see a 0 px viewport and render no rows.
export function mockElementLayout(height = 800, width = 1200): () => void {
  const clientHeight = vi
    .spyOn(HTMLElement.prototype, 'clientHeight', 'get')
    .mockReturnValue(height)
  const clientWidth = vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(width)
  const offsetHeight = vi
    .spyOn(HTMLElement.prototype, 'offsetHeight', 'get')
    .mockReturnValue(height)
  const offsetWidth = vi.spyOn(HTMLElement.prototype, 'offsetWidth', 'get').mockReturnValue(width)
  return () => {
    for (const spy of [clientHeight, clientWidth, offsetHeight, offsetWidth]) spy.mockRestore()
  }
}
