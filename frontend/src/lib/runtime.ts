export const TAURI_TITLEBAR_HEIGHT = 36

export function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && Boolean(window.__TAURI_IPC__)
}

export function getTopChromeOffset(): number {
  return isTauriRuntime() ? TAURI_TITLEBAR_HEIGHT : 0
}

