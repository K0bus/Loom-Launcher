import { ipc, type GpuBackendInfo } from '@/services/ipc'

export interface WebGLDiagnosticInfo {
  supported: boolean
  renderer: string
  vendor: string
  webglVersion: string
  maxTextureSize: number
  isSoftwareRenderer: boolean
  shadingLanguageVersion: string
  error?: string
}

export interface FpsBenchmarkResult {
  fps: number
  avgFrameTimeMs: number
  totalFrames: number
  durationMs: number
}

export interface FullGpuDiagnostic {
  timestamp: number
  webgl: WebGLDiagnosticInfo
  backend: GpuBackendInfo
  benchmark?: FpsBenchmarkResult
  isEffectiveGpuAccelerated: boolean
  effectiveGpuName: string
  isAppleGpuMasked: boolean
  diagnosticMessage: string
  statusVariant: 'success' | 'warning' | 'info'
}

/**
 * Inspects real WebGL rendering context from the current webview.
 */
export function probeWebGLContext(): WebGLDiagnosticInfo {
  if (typeof document === 'undefined') {
    return {
      supported: false,
      renderer: 'Non disponible (SSR)',
      vendor: 'Inconnu',
      webglVersion: 'N/A',
      maxTextureSize: 0,
      isSoftwareRenderer: false,
      shadingLanguageVersion: 'N/A'
    }
  }

  try {
    const canvas = document.createElement('canvas')
    canvas.width = 1
    canvas.height = 1
    const gl =
      canvas.getContext('webgl2', { powerPreference: 'high-performance' }) ||
      canvas.getContext('webgl', { powerPreference: 'high-performance' })

    if (!gl) {
      return {
        supported: false,
        renderer: 'WebGL désactivé ou indisponible',
        vendor: 'Inconnu',
        webglVersion: 'N/A',
        maxTextureSize: 0,
        isSoftwareRenderer: true,
        shadingLanguageVersion: 'N/A',
        error: 'Impossible d’initialiser le contexte WebGL'
      }
    }

    const ext = gl.getExtension('WEBGL_debug_renderer_info')
    const renderer = ext
      ? gl.getParameter(ext.UNMASKED_RENDERER_WEBGL) || gl.getParameter(gl.RENDERER) || 'Inconnu'
      : gl.getParameter(gl.RENDERER) || 'Inconnu'

    const vendor = ext
      ? gl.getParameter(ext.UNMASKED_VENDOR_WEBGL) || gl.getParameter(gl.VENDOR) || 'Inconnu'
      : gl.getParameter(gl.VENDOR) || 'Inconnu'

    const maxTextureSize = gl.getParameter(gl.MAX_TEXTURE_SIZE) || 0
    const shadingLanguageVersion = gl.getParameter(gl.SHADING_LANGUAGE_VERSION) || 'N/A'
    const webglVersion = gl instanceof WebGL2RenderingContext ? 'WebGL 2.0' : 'WebGL 1.0'

    // Detection du rendu logiciel Mesa llvmpipe/softpipe/swiftshader
    const isSoftwareRenderer =
      /llvmpipe|softpipe|software|swiftshader|cairo|mesa\s+offscreen/i.test(renderer) ||
      /mesa\s+software/i.test(vendor)

    // Nettoyage context
    const loseContextExt = gl.getExtension('WEBGL_lose_context')
    if (loseContextExt) {
      loseContextExt.loseContext()
    }

    return {
      supported: true,
      renderer: String(renderer),
      vendor: String(vendor),
      webglVersion,
      maxTextureSize: Number(maxTextureSize),
      isSoftwareRenderer,
      shadingLanguageVersion: String(shadingLanguageVersion)
    }
  } catch (err: any) {
    return {
      supported: false,
      renderer: 'Erreur détection WebGL',
      vendor: 'Inconnu',
      webglVersion: 'N/A',
      maxTextureSize: 0,
      isSoftwareRenderer: true,
      shadingLanguageVersion: 'N/A',
      error: err?.message || String(err)
    }
  }
}

/**
 * Runs a lightweight framerate benchmark using requestAnimationFrame.
 */
export function runFpsBenchmark(durationMs: number = 1000): Promise<FpsBenchmarkResult> {
  return new Promise((resolve) => {
    if (typeof window === 'undefined' || typeof requestAnimationFrame === 'undefined') {
      return resolve({
        fps: 60,
        avgFrameTimeMs: 16.6,
        totalFrames: 60,
        durationMs
      })
    }

    let frames = 0
    let startTime = performance.now()
    let lastTime = startTime
    const frameDeltas: number[] = []

    function tick(now: number) {
      frames++
      frameDeltas.push(now - lastTime)
      lastTime = now

      const elapsed = now - startTime
      if (elapsed >= durationMs) {
        const actualDuration = elapsed
        const fps = Math.round((frames / actualDuration) * 1000)
        const avgFrameTimeMs =
          Math.round((frameDeltas.reduce((a, b) => a + b, 0) / frameDeltas.length) * 10) / 10

        resolve({
          fps,
          avgFrameTimeMs,
          totalFrames: frames,
          durationMs: Math.round(actualDuration)
        })
      } else {
        requestAnimationFrame(tick)
      }
    }

    requestAnimationFrame(tick)
  })
}

/**
 * Collects full GPU diagnostic combining hardware probe and Tauri backend flags.
 */
export async function getCompleteGpuDiagnostic(withBenchmark = false): Promise<FullGpuDiagnostic> {
  const webgl = probeWebGLContext()
  let backend: GpuBackendInfo

  try {
    backend = await ipc.getGpuBackendInfo()
  } catch {
    backend = {
      hardwareAccelerationEnabled: true,
      displayServer: 'Inconnu',
      webkitDmabufDisabled: false,
      webkitCompositingDisabled: false,
      libglAlwaysSoftware: false,
      activeMode: 'hardware_accelerated'
    }
  }

  let benchmark: FpsBenchmarkResult | undefined
  if (withBenchmark) {
    benchmark = await runFpsBenchmark(800)
  }

  const isAppleGpuMasked =
    /apple\s+gpu/i.test(webgl.renderer) || /apple\s+inc/i.test(webgl.vendor)

  const isEffectiveGpuAccelerated =
    backend.hardwareAccelerationEnabled &&
    !backend.libglAlwaysSoftware &&
    webgl.supported &&
    !webgl.isSoftwareRenderer

  // Résolution du nom réel du GPU en contournant le masque anti-fingerprinting de WebKit
  let effectiveGpuName = webgl.renderer
  if (!backend.hardwareAccelerationEnabled || backend.libglAlwaysSoftware || webgl.isSoftwareRenderer) {
    effectiveGpuName = 'llvmpipe (Mesa CPU Rasterizer - Safe Mode)'
  } else if (backend.hardwareGpuName) {
    effectiveGpuName = backend.hardwareGpuName
  } else if (isAppleGpuMasked) {
    effectiveGpuName = 'GPU Matériel Détecté (Compositeur WebKit)'
  }

  let diagnosticMessage = ''
  let statusVariant: 'success' | 'warning' | 'info' = 'info'

  if (isEffectiveGpuAccelerated) {
    statusVariant = 'success'
    if (backend.hardwareGpuName && isAppleGpuMasked) {
      diagnosticMessage = `Accélération matérielle active via ${backend.hardwareGpuName}. Le masque anti-fingerprinting standard de WebKit ('Apple GPU') a été démasqué par la sonde native Linux.`
    } else {
      diagnosticMessage = `Accélération matérielle active via ${effectiveGpuName}. Compositeur matériel EGL / DMA-BUF opérationnel.`
    }
  } else if (!backend.hardwareAccelerationEnabled || backend.libglAlwaysSoftware) {
    statusVariant = 'warning'
    diagnosticMessage = `Mode sécurisé logiciel actif (${effectiveGpuName}). Rendu CPU (llvmpipe) pour une compatibilité maximale.`
  } else if (webgl.isSoftwareRenderer) {
    statusVariant = 'warning'
    diagnosticMessage = `Attention : L'accélération matérielle est demandée mais le pilote graphique a basculé sur le rasterizer CPU (${effectiveGpuName}).`
  } else {
    statusVariant = 'info'
    diagnosticMessage = `Statut graphique standard : ${effectiveGpuName}`
  }

  return {
    timestamp: Date.now(),
    webgl,
    backend,
    benchmark,
    isEffectiveGpuAccelerated,
    effectiveGpuName,
    isAppleGpuMasked,
    diagnosticMessage,
    statusVariant
  }
}
