import { describe, it, expect, vi, beforeEach } from 'vitest'

describe('SidecarBridge', () => {
  beforeEach(() => {
    vi.resetModules()
  })

  it('getServiceStatus returns service status', async () => {
    const mockInvoke = vi.fn().mockResolvedValue({
      apiServer: true,
      agentRunner: false,
    })
    vi.doMock('@tauri-apps/api/core', () => ({ invoke: mockInvoke }))

    const { getServiceStatus } = await import('../sidecar-bridge')
    const status = await getServiceStatus()
    expect(status).toEqual({ apiServer: true, agentRunner: false })
    expect(mockInvoke).toHaveBeenCalledWith('get_service_status')
  })

  it('restartApiServer invokes restart command', async () => {
    const mockInvoke = vi.fn().mockResolvedValue('API server restarted')
    vi.doMock('@tauri-apps/api/core', () => ({ invoke: mockInvoke }))

    const { restartApiServer } = await import('../sidecar-bridge')
    const result = await restartApiServer()
    expect(result).toBe('API server restarted')
    expect(mockInvoke).toHaveBeenCalledWith('restart_api_server')
  })
})
