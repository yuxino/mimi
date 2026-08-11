import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'

void cleanupLegacyPwaOnce()

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)

const legacyPwaCleanupKey = 'mimi:legacy-pwa-cleanup:v1'

async function cleanupLegacyPwaOnce() {
  if (typeof window === 'undefined') return

  try {
    if (window.localStorage.getItem(legacyPwaCleanupKey) === 'done') return
  } catch {
    // Continue when storage is unavailable so legacy registrations still get cleaned up.
  }

  document
    .querySelectorAll('link[rel="manifest"]')
    .forEach((node) => node.parentNode?.removeChild(node))

  if ('serviceWorker' in navigator) {
    const registrations = await navigator.serviceWorker.getRegistrations()
    await Promise.all(registrations.map((registration) => registration.unregister()))
  }

  if ('caches' in window) {
    const cacheKeys = await caches.keys()
    await Promise.all(cacheKeys.map((cacheKey) => caches.delete(cacheKey)))
  }

  try {
    window.localStorage.setItem(legacyPwaCleanupKey, 'done')
  } catch {
    // Storage may be disabled in private browsing; cleanup has still completed.
  }
}
