import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'

export interface ServiceStatus {
  apiServer: boolean
  agentRunner: boolean
}

export async function getServiceStatus(): Promise<ServiceStatus> {
  return invoke<ServiceStatus>('get_service_status')
}

export async function restartApiServer(): Promise<string> {
  return invoke<string>('restart_api_server')
}

export async function onServiceLog(
  callback: (line: string) => void,
): Promise<() => void> {
  const unlisten = await listen<string>('sidecar-log', (event) => {
    callback(event.payload)
  })
  return unlisten
}
