import { getCurrentWindow } from '@tauri-apps/api/window'

export function setupTitlebar(): void {
  const appWindow = getCurrentWindow()

  document.querySelectorAll('[data-tauri-drag-region]').forEach((el) => {
    el.addEventListener('mousedown', () => {
      appWindow.startDragging()
    })
  })

  const minimizeBtn = document.getElementById('titlebar-minimize')
  minimizeBtn?.addEventListener('click', () => appWindow.minimize())

  const maximizeBtn = document.getElementById('titlebar-maximize')
  maximizeBtn?.addEventListener('click', () => appWindow.toggleMaximize())

  const closeBtn = document.getElementById('titlebar-close')
  closeBtn?.addEventListener('click', () => appWindow.hide())
}
