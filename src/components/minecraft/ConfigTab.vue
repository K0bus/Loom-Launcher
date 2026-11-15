<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import {
  FileCode,
  FolderOpen,
  Save,
  RotateCcw,
  Search,
  Check,
  AlertCircle,
  AlertTriangle,
  Loader2
} from 'lucide-vue-next'
import { useMinecraftStore } from '@/stores/minecraft'
import UiBadge from '@/components/ui/UiBadge.vue'
import UiButton from '@/components/ui/UiButton.vue'

const minecraftStore = useMinecraftStore()
const activeInst = computed(() => minecraftStore.activeInstance)

const isLoading = ref(false)
const configs = ref<any[]>([])
const searchQuery = ref('')
const selectedExtension = ref<'ALL' | 'toml' | 'json' | 'yaml' | 'cfg'>('ALL')

const activeFile = ref<any | null>(null)
const fileContent = ref('')
const originalContent = ref('')
const isSaving = ref(false)
const saveSuccess = ref(false)
const isLoadingContent = ref(false)

const preRef = ref<HTMLPreElement | null>(null)
const textareaRef = ref<HTMLTextAreaElement | null>(null)
const gutterRef = ref<HTMLDivElement | null>(null)

async function loadConfigs() {
  if (!activeInst.value) return
  isLoading.value = true
  try {
    const list = await minecraftStore.getInstanceConfigs(activeInst.value.id)
    configs.value = list || []
    if (list.length > 0 && !activeFile.value) {
      selectFile(list[0])
    }
  } catch (err) {
    console.warn('[ConfigTab] Error loading configs:', err)
  } finally {
    isLoading.value = false
  }
}

async function selectFile(file: any) {
  if (isDirty.value && !confirm('Vous avez des modifications non enregistrées. Voulez-vous continuer et abandonner ces changements ?')) {
    return
  }
  if (!activeInst.value) return
  activeFile.value = file
  isLoadingContent.value = true
  saveSuccess.value = false
  try {
    const res = await minecraftStore.readInstanceConfig(activeInst.value.id, file.relPath)
    fileContent.value = res.content || ''
    originalContent.value = res.content || ''
  } catch (err: any) {
    fileContent.value = `# Erreur lors du chargement : ${err?.message || err}`
    originalContent.value = fileContent.value
  } finally {
    isLoadingContent.value = false
  }
}

const isDirty = computed(() => fileContent.value !== originalContent.value)

const filteredConfigs = computed(() => {
  let list = configs.value
  if (selectedExtension.value !== 'ALL') {
    if (selectedExtension.value === 'json') {
      list = list.filter((c) => c.extension === 'json' || c.extension === 'json5')
    } else if (selectedExtension.value === 'yaml') {
      list = list.filter((c) => c.extension === 'yaml' || c.extension === 'yml')
    } else {
      list = list.filter((c) => c.extension === selectedExtension.value)
    }
  }
  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase().trim()
    list = list.filter((c) => c.relPath.toLowerCase().includes(q) || c.name.toLowerCase().includes(q))
  }
  return list
})

// ==========================================
// REAL-TIME LINTER & SYNTAX VALIDATION
// ==========================================
interface LintResult {
  valid: boolean
  error?: string
  line?: number
  column?: number
}

function lintJson(code: string): LintResult {
  try {
    JSON.parse(code)
    return { valid: true }
  } catch (err: any) {
    const msg = err.message || ''
    const match = msg.match(/at position (\d+)/i) || msg.match(/line (\d+) column (\d+)/i)
    let line = 1
    let col = 1
    if (match) {
      if (match[2]) {
        line = parseInt(match[1], 10)
        col = parseInt(match[2], 10)
      } else {
        const pos = parseInt(match[1], 10)
        const linesBefore = code.slice(0, pos).split('\n')
        line = linesBefore.length
        col = linesBefore[linesBefore.length - 1].length + 1
      }
    }
    return { valid: false, error: msg, line, column: col }
  }
}

function lintToml(code: string): LintResult {
  const lines = code.split('\n')
  let inMultilineBasic = false
  let inMultilineLiteral = false

  for (let i = 0; i < lines.length; i++) {
    const lineNum = i + 1
    const rawLine = lines[i]
    const trimmed = rawLine.trim()

    // Handle multiline strings
    if (trimmed.includes('"""')) {
      const occurrences = (rawLine.match(/"""/g) || []).length
      if (occurrences % 2 !== 0) inMultilineBasic = !inMultilineBasic
      continue
    }
    if (trimmed.includes("'''")) {
      const occurrences = (rawLine.match(/'''/g) || []).length
      if (occurrences % 2 !== 0) inMultilineLiteral = !inMultilineLiteral
      continue
    }
    if (inMultilineBasic || inMultilineLiteral) continue

    // Comments and empty lines
    if (!trimmed || trimmed.startsWith('#')) continue

    // Table headers [table] or [[array_of_tables]]
    if (trimmed.startsWith('[')) {
      if (!trimmed.endsWith(']')) {
        return { valid: false, error: 'En-tête de table TOML non refermé (crochet "]" manquant)', line: lineNum }
      }
      continue
    }

    // Key-value pair
    if (!trimmed.includes('=')) {
      return { valid: false, error: 'Ligne invalide : séparateur clé/valeur "=" manquant', line: lineNum }
    }

    const eqIdx = trimmed.indexOf('=')
    const key = trimmed.slice(0, eqIdx).trim()
    const val = trimmed.slice(eqIdx + 1).trim()

    if (!key) {
      return { valid: false, error: 'Clé TOML manquante avant le signe "="', line: lineNum }
    }

    // Check unclosed quotes in values
    if (val.startsWith('"') && !val.endsWith('"') && val.length > 1) {
      return { valid: false, error: 'Chaîne de caractères TOML non fermée (guillemet manquant)', line: lineNum }
    }
    if (val.startsWith("'") && !val.endsWith("'") && val.length > 1) {
      return { valid: false, error: 'Chaîne littérale TOML non fermée (apostrophe manquante)', line: lineNum }
    }
  }

  return { valid: true }
}

function lintYaml(code: string): LintResult {
  const lines = code.split('\n')
  for (let i = 0; i < lines.length; i++) {
    const lineNum = i + 1
    const line = lines[i]
    if (line.includes('\t')) {
      return { valid: false, error: 'Tabulation interdite en YAML (utilisez des espaces pour l\'indentation)', line: lineNum }
    }
    const trimmed = line.trim()
    if (trimmed && !trimmed.startsWith('#')) {
      const quoteCount = (trimmed.match(/"/g) || []).length
      if (quoteCount % 2 !== 0) {
        return { valid: false, error: 'Guillemet non fermé dans la valeur YAML', line: lineNum }
      }
    }
  }
  return { valid: true }
}

function lintCfg(code: string): LintResult {
  const lines = code.split('\n')
  let openBraces = 0
  let lastOpenLine = 1

  for (let i = 0; i < lines.length; i++) {
    const lineNum = i + 1
    const trimmed = lines[i].trim()

    if (!trimmed || trimmed.startsWith('#') || trimmed.startsWith('//') || trimmed.startsWith(';')) {
      continue
    }

    if (trimmed.endsWith('{')) {
      openBraces++
      lastOpenLine = lineNum
      continue
    }
    if (trimmed === '}') {
      openBraces--
      if (openBraces < 0) {
        return { valid: false, error: 'Accolade fermante "}" inattendue sans catégorie parente', line: lineNum }
      }
      continue
    }

    const quoteCount = (trimmed.match(/"/g) || []).length
    if (quoteCount % 2 !== 0) {
      return { valid: false, error: 'Guillemet non refermé dans l\'assignation de configuration', line: lineNum }
    }
  }

  if (openBraces > 0) {
    return { valid: false, error: `Catégorie non fermée (accolade "}" manquante ouverte à la ligne ${lastOpenLine})`, line: lastOpenLine }
  }

  return { valid: true }
}

const currentLint = computed<LintResult>(() => {
  if (!activeFile.value || !fileContent.value) return { valid: true }
  const ext = activeFile.value.extension?.toLowerCase()
  if (ext === 'json' || ext === 'json5') {
    return lintJson(fileContent.value)
  }
  if (ext === 'toml') {
    return lintToml(fileContent.value)
  }
  if (ext === 'yaml' || ext === 'yml') {
    return lintYaml(fileContent.value)
  }
  if (ext === 'cfg' || ext === 'ini' || ext === 'properties') {
    return lintCfg(fileContent.value)
  }
  return { valid: true }
})

// ==========================================
// ZERO-DRIFT SYNTAX HIGHLIGHTING ENGINE
// ==========================================
function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
}

function splitValueAndComment(str: string): { valStr: string; commentStr: string } {
  let inQuote: '"' | "'" | null = null
  let commentIdx = -1

  for (let i = 0; i < str.length; i++) {
    const ch = str[i]
    if (inQuote) {
      if (ch === inQuote && str[i - 1] !== '\\') {
        inQuote = null
      }
    } else {
      if (ch === '"' || ch === "'") {
        inQuote = ch
      } else if (ch === '#') {
        commentIdx = i
        break
      }
    }
  }

  if (commentIdx !== -1) {
    return {
      valStr: str.slice(0, commentIdx),
      commentStr: str.slice(commentIdx)
    }
  }
  return { valStr: str, commentStr: '' }
}

function highlightValueToken(valStr: string): string {
  if (!valStr) return ''

  const tokens: string[] = []
  let lastIdx = 0
  const regex = /("(?:\\[\s\S]|[^"\\])*"|'(?:\\[\s\S]|[^'\\])*'|-?\b(?:0x[0-9a-fA-F]+|\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)\b|\b(?:true|false|null|yes|no|on|off)\b|[\[\]{},])/gi
  let match: RegExpExecArray | null

  while ((match = regex.exec(valStr)) !== null) {
    if (match.index > lastIdx) {
      tokens.push(escapeHtml(valStr.slice(lastIdx, match.index)))
    }
    const m = match[0]
    if (m.startsWith('"') || m.startsWith("'")) {
      tokens.push(`<span class="text-emerald-400">${escapeHtml(m)}</span>`)
    } else if (/^(true|false|null|yes|no|on|off)$/i.test(m)) {
      tokens.push(`<span class="text-purple-400">${escapeHtml(m)}</span>`)
    } else if (/^-?(?:0x[0-9a-fA-F]+|\d)/.test(m)) {
      tokens.push(`<span class="text-cyan-400">${escapeHtml(m)}</span>`)
    } else if (/[\[\]{},]/.test(m)) {
      tokens.push(`<span class="text-amber-400/80">${escapeHtml(m)}</span>`)
    } else {
      tokens.push(escapeHtml(m))
    }
    lastIdx = regex.lastIndex
  }

  if (lastIdx < valStr.length) {
    tokens.push(escapeHtml(valStr.slice(lastIdx)))
  }

  return tokens.join('')
}

function highlightTomlLine(rawLine: string): string {
  if (!rawLine) return '&nbsp;'

  const trimmed = rawLine.trim()

  if (trimmed.startsWith('#')) {
    return `<span class="text-zinc-500">${escapeHtml(rawLine)}</span>`
  }

  const tableMatch = rawLine.match(/^(\s*)(\[+)([^\]]+)(\]+)(.*)$/)
  if (tableMatch) {
    const [, indent, open, name, close, rest] = tableMatch
    let restHtml = ''
    if (rest) {
      const { valStr, commentStr } = splitValueAndComment(rest)
      restHtml = escapeHtml(valStr) + (commentStr ? `<span class="text-zinc-500">${escapeHtml(commentStr)}</span>` : '')
    }
    return (
      escapeHtml(indent) +
      `<span class="text-amber-400">${escapeHtml(open)}</span>` +
      `<span class="text-amber-300">${escapeHtml(name)}</span>` +
      `<span class="text-amber-400">${escapeHtml(close)}</span>` +
      restHtml
    )
  }

  const eqIdx = rawLine.indexOf('=')
  if (eqIdx !== -1 && !trimmed.startsWith('#')) {
    const rawKey = rawLine.slice(0, eqIdx)
    const rawValAndComment = rawLine.slice(eqIdx + 1)

    const keyParts = rawKey.split('.')
    const highlightedKey = keyParts
      .map((p) => `<span class="text-sky-300">${escapeHtml(p)}</span>`)
      .join('<span class="text-zinc-500">.</span>')

    const { valStr, commentStr } = splitValueAndComment(rawValAndComment)
    const highlightedVal = highlightValueToken(valStr)
    const highlightedComment = commentStr ? `<span class="text-zinc-500">${escapeHtml(commentStr)}</span>` : ''

    return (
      highlightedKey +
      `<span class="text-zinc-500">=</span>` +
      highlightedVal +
      highlightedComment
    )
  }

  return escapeHtml(rawLine)
}

function highlightJsonLine(rawLine: string): string {
  if (!rawLine) return '&nbsp;'

  const trimmed = rawLine.trim()

  if (trimmed.startsWith('//') || trimmed.startsWith('/*') || trimmed.startsWith('*')) {
    return `<span class="text-zinc-500">${escapeHtml(rawLine)}</span>`
  }

  const colonMatch = rawLine.match(/^(\s*)("([^"]+)"|'([^']+)'|([a-zA-Z0-9_$]+))(\s*:\s*)(.*)$/)
  if (colonMatch) {
    const [, indent, rawKey, , , , colon, rest] = colonMatch
    const highlightedKey = `<span class="text-sky-300">${escapeHtml(rawKey)}</span>`
    const highlightedColon = `<span class="text-zinc-500">${escapeHtml(colon)}</span>`
    const highlightedVal = highlightValueToken(rest)

    return escapeHtml(indent) + highlightedKey + highlightedColon + highlightedVal
  }

  return highlightValueToken(rawLine)
}

function highlightYamlLine(rawLine: string): string {
  if (!rawLine) return '&nbsp;'

  const trimmed = rawLine.trim()

  if (trimmed.startsWith('#')) {
    return `<span class="text-zinc-500">${escapeHtml(rawLine)}</span>`
  }

  if (trimmed === '---' || trimmed === '...') {
    return `<span class="text-amber-500">${escapeHtml(rawLine)}</span>`
  }

  const bulletMatch = rawLine.match(/^(\s*)(-\s+)(.*)$/)
  let indent = ''
  let bullet = ''
  let content = rawLine

  if (bulletMatch) {
    indent = bulletMatch[1]
    bullet = `<span class="text-amber-400">${escapeHtml(bulletMatch[2])}</span>`
    content = bulletMatch[3]
  }

  const colonIdx = content.indexOf(':')
  if (colonIdx !== -1 && !content.trim().startsWith('#')) {
    const rawKey = content.slice(0, colonIdx)
    const afterColon = content.slice(colonIdx + 1)
    const { valStr, commentStr } = splitValueAndComment(afterColon)

    const highlightedKey = `<span class="text-sky-300">${escapeHtml(rawKey)}</span>`
    const highlightedVal = highlightValueToken(valStr)
    const highlightedComment = commentStr ? `<span class="text-zinc-500">${escapeHtml(commentStr)}</span>` : ''

    return (
      escapeHtml(indent) +
      bullet +
      highlightedKey +
      `<span class="text-zinc-500">:</span>` +
      highlightedVal +
      highlightedComment
    )
  }

  return escapeHtml(indent) + bullet + highlightValueToken(content)
}

function highlightCfgLine(rawLine: string): string {
  if (!rawLine) return '&nbsp;'

  const trimmed = rawLine.trim()

  if (trimmed.startsWith('#') || trimmed.startsWith('//') || trimmed.startsWith(';')) {
    return `<span class="text-zinc-500">${escapeHtml(rawLine)}</span>`
  }

  const catMatch = rawLine.match(/^(\s*)(.+?)(\s*\{\s*)$/)
  if (catMatch) {
    return (
      escapeHtml(catMatch[1]) +
      `<span class="text-amber-400">${escapeHtml(catMatch[2])}</span>` +
      `<span class="text-amber-400">${escapeHtml(catMatch[3])}</span>`
    )
  }

  if (trimmed === '}') {
    const closeMatch = rawLine.match(/^(\s*)(\})(\s*)$/)
    if (closeMatch) {
      return escapeHtml(closeMatch[1]) + `<span class="text-amber-400">}</span>` + escapeHtml(closeMatch[3])
    }
  }

  const typedMatch = rawLine.match(/^(\s*)([SIBDF]:)("([^"]+)"|([a-zA-Z0-9_$-]+))(\s*=\s*)(.*)$/)
  if (typedMatch) {
    const [, indent, type, fullKey, , , eq, rest] = typedMatch
    const { valStr, commentStr } = splitValueAndComment(rest)
    const highlightedType = `<span class="text-rose-400">${escapeHtml(type)}</span>`
    const highlightedKey = `<span class="text-sky-300">${escapeHtml(fullKey)}</span>`
    const highlightedVal = highlightValueToken(valStr)
    const highlightedComment = commentStr ? `<span class="text-zinc-500">${escapeHtml(commentStr)}</span>` : ''

    return (
      escapeHtml(indent) +
      highlightedType +
      highlightedKey +
      `<span class="text-zinc-500">${escapeHtml(eq)}</span>` +
      highlightedVal +
      highlightedComment
    )
  }

  const eqIdx = rawLine.indexOf('=')
  if (eqIdx !== -1) {
    const rawKey = rawLine.slice(0, eqIdx)
    const rest = rawLine.slice(eqIdx + 1)
    const { valStr, commentStr } = splitValueAndComment(rest)

    return (
      `<span class="text-sky-300">${escapeHtml(rawKey)}</span>` +
      `<span class="text-zinc-500">=</span>` +
      highlightValueToken(valStr) +
      (commentStr ? `<span class="text-zinc-500">${escapeHtml(commentStr)}</span>` : '')
    )
  }

  return highlightValueToken(rawLine)
}

function highlightGenericLine(rawLine: string): string {
  if (!rawLine) return '&nbsp;'

  const trimmed = rawLine.trim()

  if (trimmed.startsWith('#') || trimmed.startsWith('//') || trimmed.startsWith(';')) {
    return `<span class="text-zinc-500">${escapeHtml(rawLine)}</span>`
  }

  return highlightValueToken(rawLine)
}

function highlightLine(rawLine: string, ext: string): string {
  if (ext === 'toml') return highlightTomlLine(rawLine)
  if (ext === 'json' || ext === 'json5') return highlightJsonLine(rawLine)
  if (ext === 'yaml' || ext === 'yml') return highlightYamlLine(rawLine)
  if (ext === 'cfg' || ext === 'ini' || ext === 'properties') return highlightCfgLine(rawLine)
  return highlightGenericLine(rawLine)
}

const highlightedLines = computed(() => {
  if (!fileContent.value) return ['&nbsp;']
  const ext = activeFile.value?.extension?.toLowerCase() || ''
  const lines = fileContent.value.split('\n')
  const errLine = !currentLint.value.valid ? currentLint.value.line : null
  const errMsg = currentLint.value.error || ''

  return lines.map((line, idx) => {
    const lineNum = idx + 1
    const isError = errLine === lineNum
    let html = highlightLine(line, ext)
    if (!html.trim()) {
      html = '&nbsp;'
    }

    if (isError) {
      html = `<span class="ide-error-wave" title="${escapeHtml(errMsg)}">${html}</span>`
    }
    return html
  })
})

function handleScroll() {
  if (!textareaRef.value) return
  const top = textareaRef.value.scrollTop
  const left = textareaRef.value.scrollLeft

  if (preRef.value) {
    preRef.value.scrollTop = top
    preRef.value.scrollLeft = left
  }
  if (gutterRef.value) {
    gutterRef.value.scrollTop = top
  }
}

function handleGutterWheel(e: WheelEvent) {
  if (textareaRef.value) {
    textareaRef.value.scrollTop += e.deltaY
  }
}

function goToErrorLine(targetLine?: number) {
  const lineToJump = targetLine || currentLint.value.line
  if (!lineToJump || !textareaRef.value) return

  const lines = fileContent.value.split('\n')
  let charPos = 0
  for (let i = 0; i < lineToJump - 1 && i < lines.length; i++) {
    charPos += lines[i].length + 1
  }
  if (currentLint.value.column && lineToJump === currentLint.value.line) {
    charPos += Math.min(currentLint.value.column - 1, lines[lineToJump - 1]?.length || 0)
  }

  textareaRef.value.focus()
  const lineLength = lines[lineToJump - 1]?.length || 0
  textareaRef.value.setSelectionRange(charPos, charPos + lineLength)

  const lineHeight = 22
  textareaRef.value.scrollTop = Math.max(0, (lineToJump - 5) * lineHeight)
  if (preRef.value) preRef.value.scrollTop = textareaRef.value.scrollTop
  if (gutterRef.value) gutterRef.value.scrollTop = textareaRef.value.scrollTop
}

async function saveFile() {
  if (!activeInst.value || !activeFile.value) return
  if (!currentLint.value.valid) {
    const proceed = confirm(
      `⚠️ Attention : Une erreur de syntaxe a été détectée à la ligne ${currentLint.value.line || 'inconnue'} :\n"${currentLint.value.error}"\n\nEnregistrer une configuration erronée peut empêcher le mod ou Minecraft de démarrer.\nVoulez-vous quand même enregistrer ?`
    )
    if (!proceed) return
  }

  isSaving.value = true
  saveSuccess.value = false
  try {
    await minecraftStore.saveInstanceConfig(activeInst.value.id, activeFile.value.relPath, fileContent.value)
    originalContent.value = fileContent.value
    saveSuccess.value = true
    setTimeout(() => (saveSuccess.value = false), 2500)
  } catch (err: any) {
    alert(`Erreur lors de la sauvegarde : ${err?.message || err}`)
  } finally {
    isSaving.value = false
  }
}

function revertChanges() {
  if (confirm('Rétablir la version enregistrée sur disque et annuler vos modifications ?')) {
    fileContent.value = originalContent.value
  }
}

function openConfigFolder() {
  if (activeInst.value) {
    minecraftStore.openInstanceFolder(activeInst.value.id, 'config')
  }
}

const lineCount = computed(() => fileContent.value.split('\n').length)

function handleKeydown(e: KeyboardEvent) {
  if ((e.ctrlKey || e.metaKey) && e.key === 's') {
    e.preventDefault()
    if (isDirty.value && !isSaving.value) {
      saveFile()
    }
  }
}

function handleEditorKeydown(e: KeyboardEvent) {
  if (e.key === 'Tab') {
    e.preventDefault()
    const target = e.target as HTMLTextAreaElement
    const start = target.selectionStart
    const end = target.selectionEnd

    if (!e.shiftKey) {
      fileContent.value = fileContent.value.substring(0, start) + '  ' + fileContent.value.substring(end)
      setTimeout(() => {
        target.selectionStart = target.selectionEnd = start + 2
      }, 0)
    } else {
      const beforeCursor = fileContent.value.substring(0, start)
      const lastNewline = beforeCursor.lastIndexOf('\n')
      const lineStart = lastNewline === -1 ? 0 : lastNewline + 1
      const linePrefix = fileContent.value.substring(lineStart, lineStart + 2)

      if (linePrefix === '  ') {
        fileContent.value = fileContent.value.substring(0, lineStart) + fileContent.value.substring(lineStart + 2)
        setTimeout(() => {
          target.selectionStart = target.selectionEnd = Math.max(lineStart, start - 2)
        }, 0)
      }
    }
  }
}

onMounted(() => {
  loadConfigs()
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
})
</script>

<template>
  <div class="space-y-6">
    <!-- Header Toolbar -->
    <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-4 bg-zinc-900/60 p-5 rounded-3xl border border-zinc-800/80">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-2xl bg-zinc-950 border border-zinc-800 flex items-center justify-center text-amber-400 shadow-inner">
          <FileCode class="w-5 h-5" />
        </div>
        <div>
          <div class="text-sm font-bold text-zinc-100 flex items-center gap-2">
            <span>Fichiers de Configuration des Mods</span>
            <UiBadge variant="amber" size="xs">{{ configs.length }}</UiBadge>
          </div>
          <p class="text-xs text-zinc-500 font-mono">Dossier <span class="text-zinc-400">.minecraft/config/</span> avec coloration syntaxique & linter temps réel</p>
        </div>
      </div>

      <div class="flex items-center gap-2.5">
        <UiButton variant="ghost" size="sm" @click="loadConfigs" :disabled="isLoading" title="Actualiser la liste">
          <RotateCcw :class="['w-3.5 h-3.5', isLoading ? 'animate-spin text-amber-400' : '']" />
        </UiButton>

        <UiButton variant="secondary" size="sm" @click="openConfigFolder" title="Ouvrir le dossier config dans l'explorateur">
          <template #prefix>
            <FolderOpen class="w-3.5 h-3.5 text-amber-400" />
          </template>
          Dossier config
        </UiButton>
      </div>
    </div>

    <!-- Main Editor Grid -->
    <div class="grid grid-cols-1 lg:grid-cols-12 gap-6">
      <!-- LEFT SIDEBAR : Config Files Explorer (4 cols) -->
      <div class="lg:col-span-4 space-y-3">
        <!-- Search & Extension Filters -->
        <div class="p-3 bg-zinc-900/60 rounded-2xl border border-zinc-800 space-y-2.5">
          <div class="relative">
            <input
              v-model="searchQuery"
              type="text"
              placeholder="Rechercher un fichier de config..."
              class="w-full bg-zinc-950 border border-zinc-800 rounded-xl pl-8 pr-3 py-1.5 text-xs text-zinc-200 placeholder-zinc-500 focus:outline-none focus:border-amber-500"
            />
            <Search class="w-3.5 h-3.5 text-zinc-500 absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
          </div>

          <!-- Type Filters -->
          <div class="flex flex-wrap items-center gap-1 text-[10px] font-mono">
            <button
              v-for="ext in (['ALL', 'toml', 'json', 'yaml', 'cfg'] as const)"
              :key="ext"
              type="button"
              @click="selectedExtension = ext"
              :class="[
                'px-2 py-0.5 rounded-lg font-bold transition uppercase',
                selectedExtension === ext ? 'bg-amber-500 text-zinc-950' : 'bg-zinc-950 text-zinc-400 hover:text-zinc-200 border border-zinc-800'
              ]"
            >
              {{ ext }}
            </button>
          </div>
        </div>

        <!-- Files List -->
        <div class="h-[580px] overflow-y-auto space-y-1.5 pr-1">
          <button
            v-for="file in filteredConfigs"
            :key="file.relPath"
            type="button"
            @click="selectFile(file)"
            :class="[
              'w-full text-left p-3 rounded-2xl border transition select-none flex items-center justify-between gap-3',
              activeFile?.relPath === file.relPath
                ? 'bg-amber-500/10 border-amber-500/40 text-amber-300'
                : 'bg-zinc-900/60 border-zinc-800/80 text-zinc-300 hover:border-zinc-700 hover:bg-zinc-900'
            ]"
          >
            <div class="flex-1 min-w-0">
              <div class="text-xs font-mono font-bold truncate flex items-center gap-1.5">
                <span class="truncate">{{ file.name }}</span>
                <span v-if="activeFile?.relPath === file.relPath && isDirty" class="w-2 h-2 rounded-full bg-amber-400 shrink-0" title="Non enregistré" />
              </div>
              <p v-if="file.subfolder !== '/'" class="text-[10px] font-mono text-zinc-500 truncate mt-0.5">
                {{ file.subfolder }}
              </p>
            </div>

            <UiBadge
              :variant="file.extension === 'toml' ? 'sky' : file.extension === 'json' || file.extension === 'json5' ? 'emerald' : file.extension === 'yaml' ? 'amber' : 'zinc'"
              size="xs"
              class="shrink-0 uppercase font-mono"
            >
              {{ file.extension }}
            </UiBadge>
          </button>

          <div v-if="filteredConfigs.length === 0" class="p-8 text-center text-xs text-zinc-500 italic">
            Aucun fichier trouvé.
          </div>
        </div>
      </div>

      <!-- RIGHT PANEL : Code Editor & Linter (8 cols) -->
      <div class="lg:col-span-8 space-y-3">
        <!-- Editor Top Action Bar -->
        <div class="flex flex-col sm:flex-row items-stretch sm:items-center justify-between gap-3 bg-zinc-900/60 p-3.5 rounded-2xl border border-zinc-800">
          <div class="flex items-center gap-2 min-w-0">
            <span class="text-xs font-mono font-bold text-zinc-200 truncate">
              {{ activeFile?.relPath || 'Aucun fichier sélectionné' }}
            </span>
            <span v-if="isDirty" class="px-2 py-0.5 rounded-full bg-amber-500/20 text-amber-300 text-[10px] font-mono font-bold shrink-0">
              • Modifié
            </span>
          </div>

          <div class="flex items-center gap-2 shrink-0">
            <!-- Revert button -->
            <button
              type="button"
              @click="revertChanges"
              :disabled="!isDirty"
              class="px-2.5 py-1.5 rounded-xl bg-zinc-950 hover:bg-zinc-800 border border-zinc-800 text-xs font-mono text-zinc-400 hover:text-zinc-200 transition disabled:opacity-40"
              title="Annuler les modifications non enregistrées"
            >
              Rétablir
            </button>

            <!-- Save CTA -->
            <UiButton
              variant="primary"
              size="sm"
              @click="saveFile"
              :disabled="!isDirty || isSaving"
              title="Enregistrer (Ctrl+S)"
            >
              <Loader2 v-if="isSaving" class="w-3.5 h-3.5 animate-spin" />
              <Check v-else-if="saveSuccess" class="w-3.5 h-3.5" />
              <Save v-else class="w-3.5 h-3.5" />
              <span>{{ saveSuccess ? 'Enregistré !' : 'Enregistrer' }}</span>
            </UiButton>
          </div>
        </div>

        <!-- LINTER BANNER -->
        <div
          v-if="activeFile"
          :class="[
            'px-4 py-2.5 rounded-2xl text-xs font-mono flex items-center justify-between gap-3 transition-all select-none',
            currentLint.valid
              ? 'bg-emerald-500/10 border border-emerald-500/20 text-emerald-400'
              : 'bg-rose-500/10 border border-rose-500/30 text-rose-300 shadow-sm'
          ]"
        >
          <div class="flex items-center gap-2.5 min-w-0">
            <Check v-if="currentLint.valid" class="w-4 h-4 text-emerald-400 shrink-0" />
            <AlertCircle v-else class="w-4 h-4 text-rose-400 shrink-0 animate-pulse" />
            <div class="truncate">
              <span v-if="currentLint.valid" class="font-bold">
                Syntaxe {{ activeFile.extension.toUpperCase() }} valide
              </span>
              <span v-else class="font-bold">
                Erreur de syntaxe {{ currentLint.line ? `(ligne ${currentLint.line})` : '' }} :
                <span class="font-normal text-rose-200">{{ currentLint.error }}</span>
              </span>
            </div>
          </div>

          <div class="flex items-center gap-2 shrink-0">
            <button
              v-if="!currentLint.valid && currentLint.line"
              type="button"
              @click="goToErrorLine()"
              class="px-2.5 py-1 rounded-xl bg-rose-500/20 hover:bg-rose-500/30 border border-rose-500/40 text-rose-200 font-mono text-[11px] font-bold transition flex items-center gap-1.5"
            >
              <AlertTriangle class="w-3.5 h-3.5 text-rose-400" />
              <span>Aller à la ligne {{ currentLint.line }}</span>
            </button>
            <span class="text-[10px] text-zinc-500 hidden sm:inline">Linter temps réel</span>
          </div>
        </div>

        <!-- Code Editor Area with Line Numbers & Real-time Syntax Highlighting -->
        <div class="relative rounded-3xl bg-zinc-950 border border-zinc-800 shadow-2xl overflow-hidden flex flex-col">
          <div v-if="isLoadingContent" class="h-[500px] flex items-center justify-center">
            <Loader2 class="w-8 h-8 text-amber-500 animate-spin" />
          </div>

          <div v-else class="flex h-[500px] font-mono text-xs overflow-hidden relative">
            <!-- Line Numbers Gutter -->
            <div
              ref="gutterRef"
              @wheel="handleGutterWheel"
              class="w-14 py-4 bg-zinc-950/95 border-r border-zinc-800/80 text-zinc-600 text-right pr-2 select-none overflow-hidden shrink-0 space-y-0 text-xs font-mono code-editor-font"
            >
              <div
                v-for="n in lineCount"
                :key="n"
                @click="goToErrorLine(n)"
                :class="[
                  'gutter-line h-[22px] min-h-[22px] leading-[22px] flex items-center justify-end gap-1 px-1 transition-colors',
                  !currentLint.valid && currentLint.line === n
                    ? 'text-rose-400 font-bold bg-rose-500/20 cursor-pointer rounded'
                    : 'hover:text-zinc-400'
                ]"
                :title="!currentLint.valid && currentLint.line === n ? `Erreur ligne ${n} : ${currentLint.error}` : undefined"
              >
                <span
                  v-if="!currentLint.valid && currentLint.line === n"
                  class="w-1.5 h-1.5 rounded-full bg-rose-500 animate-pulse shrink-0"
                />
                <span>{{ n }}</span>
              </div>
            </div>

            <!-- Editor Wrapper (Pre Code Layer + Interactive Textarea Layer) -->
            <div class="relative flex-1 h-full overflow-hidden bg-zinc-950">
              <!-- Background Syntax Highlighted Code Layer -->
              <pre
                ref="preRef"
                aria-hidden="true"
                class="editor-pre absolute inset-0 m-0 py-4 px-4 code-editor-font pointer-events-none select-none overflow-hidden whitespace-pre text-zinc-300"
              ><div
                  v-for="(lineHtml, idx) in highlightedLines"
                  :key="idx"
                  class="code-line h-[22px] min-h-[22px] leading-[22px] whitespace-pre"
                  v-html="lineHtml"
                ></div></pre>

              <!-- Foreground Interactive Textarea Layer -->
              <textarea
                ref="textareaRef"
                v-model="fileContent"
                @scroll="handleScroll"
                @keydown="handleEditorKeydown"
                spellcheck="false"
                autocomplete="off"
                autocorrect="off"
                autocapitalize="off"
                class="editor-textarea absolute inset-0 m-0 py-4 px-4 code-editor-font bg-transparent resize-none outline-none whitespace-pre overflow-auto"
              ></textarea>
            </div>
          </div>

          <!-- Bottom Editor Status Bar -->
          <div class="px-4 py-2.5 bg-zinc-900/90 border-t border-zinc-800/80 flex items-center justify-between text-[11px] font-mono text-zinc-500 select-none">
            <div class="flex items-center gap-3">
              <span class="text-zinc-300 font-bold uppercase">{{ activeFile?.extension || 'TEXT' }}</span>
              <span>•</span>
              <span>{{ lineCount }} lignes</span>
              <span>•</span>
              <span>{{ fileContent.length }} caractères</span>
              <span>•</span>
              <span>UTF-8</span>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-zinc-400">
                <kbd class="px-1.5 py-0.5 rounded bg-zinc-800 text-zinc-300 font-mono">Ctrl+S</kbd> pour enregistrer
              </span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.code-editor-font {
  font-family: 'Noto Sans Mono', 'DejaVu Sans Mono', 'Liberation Mono', Consolas, ui-monospace, monospace !important;
  font-size: 13px !important;
  line-height: 22px !important;
  font-weight: 400 !important;
  font-style: normal !important;
  letter-spacing: 0px !important;
  word-spacing: 0px !important;
  tab-size: 2 !important;
  -moz-tab-size: 2 !important;
  font-variant-ligatures: none !important;
  font-feature-settings: "liga" 0, "calt" 0 !important;
  text-rendering: geometricPrecision !important;
  -webkit-font-smoothing: antialiased !important;
  -moz-osx-font-smoothing: grayscale !important;
  white-space: pre !important;
  word-break: normal !important;
  overflow-wrap: normal !important;
}

.editor-pre,
.editor-textarea {
  box-sizing: border-box !important;
  margin: 0 !important;
  border: 0 !important;
  padding: 16px !important;
  outline: none !important;
}

.editor-textarea {
  color: transparent !important;
  -webkit-text-fill-color: transparent !important;
  caret-color: #fbbf24 !important;
  background: transparent !important;
  resize: none !important;
  -webkit-appearance: none !important;
  appearance: none !important;
}

.editor-textarea::selection {
  background-color: rgba(245, 158, 11, 0.3) !important;
  color: transparent !important;
  -webkit-text-fill-color: transparent !important;
}

:deep(.ide-error-wave) {
  text-decoration-line: underline !important;
  text-decoration-style: wavy !important;
  text-decoration-color: #f43f5e !important;
  text-decoration-thickness: 1.5px !important;
  text-underline-offset: 3.5px !important;
  background-color: rgba(244, 63, 94, 0.15) !important;
  border-radius: 2px !important;
}
</style>
