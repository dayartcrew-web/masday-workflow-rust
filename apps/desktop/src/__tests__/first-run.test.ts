// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest'

describe('FirstRunWizard', () => {
  beforeEach(() => {
    localStorage.clear()
    vi.resetModules()
  })

  it('isFirstRun returns true when no key in localStorage', async () => {
    vi.doMock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn() }))
    vi.doMock('@tauri-apps/plugin-autostart', () => ({
      isEnabled: vi.fn(),
      enable: vi.fn(),
    }))
    vi.doMock('@tauri-apps/plugin-notification', () => ({
      isPermissionGranted: vi.fn().mockResolvedValue(false),
      requestPermission: vi.fn().mockResolvedValue('denied'),
      sendNotification: vi.fn(),
    }))

    const { isFirstRun } = await import('../first-run')
    expect(await isFirstRun()).toBe(true)
  })

  it('isFirstRun returns false after wizard completes', async () => {
    localStorage.setItem('masday-workflow-first-run-complete', '2026-05-16T00:00:00.000Z')

    vi.doMock('@tauri-apps/plugin-dialog', () => ({ ask: vi.fn() }))
    vi.doMock('@tauri-apps/plugin-autostart', () => ({
      isEnabled: vi.fn(),
      enable: vi.fn(),
    }))
    vi.doMock('@tauri-apps/plugin-notification', () => ({
      isPermissionGranted: vi.fn(),
      requestPermission: vi.fn(),
      sendNotification: vi.fn(),
    }))

    const { isFirstRun } = await import('../first-run')
    expect(await isFirstRun()).toBe(false)
  })
})
