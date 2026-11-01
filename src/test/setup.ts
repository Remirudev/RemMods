import '@testing-library/jest-dom/vitest'
import '../lib/i18n'

import { clearMocks } from '@tauri-apps/api/mocks'
import { cleanup } from '@testing-library/react'
import { afterEach, vi } from 'vitest'

// jsdom lacks the layout APIs Mantine touches (media queries, resize observers, scrolling), and its
// getComputedStyle throws when given a pseudo-element.
class ResizeObserverStub {
  observe = vi.fn()
  unobserve = vi.fn()
  disconnect = vi.fn()
}

Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }),
})

// Autosizing textareas listen for font loads.
Object.defineProperty(document, 'fonts', {
  configurable: true,
  value: { addEventListener: vi.fn(), removeEventListener: vi.fn() },
})

Object.defineProperty(window, 'ResizeObserver', { writable: true, value: ResizeObserverStub })

const jsdomGetComputedStyle = window.getComputedStyle.bind(window)
window.getComputedStyle = (element: Element) => jsdomGetComputedStyle(element)
window.HTMLElement.prototype.scrollIntoView = vi.fn()

afterEach(() => {
  cleanup()
  clearMocks()
})
