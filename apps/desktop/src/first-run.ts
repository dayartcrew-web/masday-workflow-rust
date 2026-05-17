import { ask } from '@tauri-apps/plugin-dialog'
import { isEnabled, enable } from '@tauri-apps/plugin-autostart'
import { sendNotification, isPermissionGranted, requestPermission } from '@tauri-apps/plugin-notification'

const FIRST_RUN_KEY = 'masday-workflow-first-run-complete'

export async function isFirstRun(): Promise<boolean> {
  return localStorage.getItem(FIRST_RUN_KEY) === null
}

export async function runFirstRunWizard(): Promise<void> {
  const autoStartChoice = await ask(
    'Would you like Masday Workflow to start automatically when you log in?',
    { title: 'Auto-Start', kind: 'info', okLabel: 'Enable', cancelLabel: 'Skip' },
  )

  if (autoStartChoice) {
    const currentlyEnabled = await isEnabled()
    if (!currentlyEnabled) {
      await enable()
    }
  }

  let permissionGranted = await isPermissionGranted()
  if (!permissionGranted) {
    const permission = await requestPermission()
    permissionGranted = permission === 'granted'
  }

  if (permissionGranted) {
    sendNotification({
      title: 'Masday Workflow',
      body: 'Setup complete! Your workflow engine is ready.',
    })
  }

  localStorage.setItem(FIRST_RUN_KEY, new Date().toISOString())
}
