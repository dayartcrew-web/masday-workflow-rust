// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest'

describe('Titlebar', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
    vi.resetModules()
  })

  it('setupTitlebar attaches drag region listeners', async () => {
    const mockStartDragging = vi.fn()
    vi.doMock('@tauri-apps/api/window', () => ({
      getCurrentWindow: () => ({
        startDragging: mockStartDragging,
        minimize: vi.fn(),
        toggleMaximize: vi.fn(),
        hide: vi.fn(),
      }),
    }))

    const dragEl = document.createElement('div')
    dragEl.setAttribute('data-tauri-drag-region', '')
    document.body.appendChild(dragEl)

    const { setupTitlebar } = await import('../titlebar')
    setupTitlebar()

    dragEl.dispatchEvent(new MouseEvent('mousedown'))
    expect(mockStartDragging).toHaveBeenCalled()
  })

  it('minimize button calls window.minimize', async () => {
    const mockMinimize = vi.fn()
    vi.doMock('@tauri-apps/api/window', () => ({
      getCurrentWindow: () => ({
        startDragging: vi.fn(),
        minimize: mockMinimize,
        toggleMaximize: vi.fn(),
        hide: vi.fn(),
      }),
    }))

    const btn = document.createElement('button')
    btn.id = 'titlebar-minimize'
    document.body.appendChild(btn)

    const { setupTitlebar } = await import('../titlebar')
    setupTitlebar()

    btn.click()
    expect(mockMinimize).toHaveBeenCalled()
  })
})
