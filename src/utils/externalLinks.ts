import { ipc } from '@/services/ipc'

let originalWindowOpen: ((url?: string | URL, target?: string, features?: string) => Window | null) | null = null

/**
 * Safely opens a URL in a new window/tab in pure browser mode without recursive interception.
 */
export function safeBrowserOpen(url: string): void {
  if (typeof window === 'undefined') return
  if (originalWindowOpen) {
    originalWindowOpen.call(window, url, '_blank', 'noopener,noreferrer')
  } else {
    window.open(url, '_blank', 'noopener,noreferrer')
  }
}

/**
 * Open any external link (http, https, steam, mailto) in the default system browser
 * using Tauri's backend or web fallback.
 */
export async function openExternalLink(url: string | null | undefined): Promise<void> {
  if (!url) return
  const trimmed = url.trim()
  if (!trimmed || trimmed === '#' || trimmed.startsWith('javascript:')) return

  try {
    await ipc.openExternalUrl(trimmed)
  } catch (err) {
    console.warn('[ExternalLinks] Échec de l\'ouverture du lien externe :', trimmed, err)
    safeBrowserOpen(trimmed)
  }
}

/**
 * Initializes global interceptors for all external link clicks and window.open calls.
 * Ensures:
 * 1. Any <a href="http..."> clicked anywhere (templates, v-html markdown, logs) opens in default system browser
 * 2. Any window.open(...) call delegates to default system browser
 * 3. Middle clicks (auxclick) on links also open in default browser
 * 4. Internal application navigation remains completely intact
 */
export function setupExternalLinksInterceptor(): void {
  if (typeof window === 'undefined' || typeof document === 'undefined') return

  // 1. Monkey-patch window.open to delegate external protocols to system browser
  if (!originalWindowOpen) {
    originalWindowOpen = window.open.bind(window)
  }

  window.open = (url?: string | URL, target?: string, features?: string) => {
    if (url) {
      const urlStr = url.toString().trim()
      if (
        urlStr.startsWith('http://') ||
        urlStr.startsWith('https://') ||
        urlStr.startsWith('steam://') ||
        urlStr.startsWith('mailto:')
      ) {
        openExternalLink(urlStr)
        return null
      }
    }
    return originalWindowOpen ? originalWindowOpen(url, target, features) : null
  }

  // 2. Global event delegation for clicks and middle-clicks on <a> tags
  const handleLinkClick = (event: MouseEvent) => {
    // Only capture primary click (0) or middle click (1)
    if (event.button !== 0 && event.button !== 1) return

    const target = event.target as HTMLElement | null
    const anchor = target?.closest('a')
    if (!anchor) return

    const href = anchor.getAttribute('href')
    if (!href) return

    const trimmed = href.trim()

    // Skip local anchors, router paths, or javascript calls
    if (
      !trimmed ||
      trimmed === '#' ||
      trimmed.startsWith('#/') ||
      trimmed.startsWith('javascript:')
    ) {
      return
    }

    // Intercept external URLs
    if (
      trimmed.startsWith('http://') ||
      trimmed.startsWith('https://') ||
      trimmed.startsWith('steam://') ||
      trimmed.startsWith('mailto:')
    ) {
      event.preventDefault()
      event.stopPropagation()
      event.stopImmediatePropagation()
      openExternalLink(trimmed)
    }
  }

  document.addEventListener('click', handleLinkClick, true)
  document.addEventListener('auxclick', handleLinkClick, true)
}
