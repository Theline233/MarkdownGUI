<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onBeforeUnmount, watch } from 'vue'
import {
  UploadFilled,
  Document,
  Tickets,
  Files,
  Download,
  ArrowDown,
  ArrowUp,
  Grid,
  MagicStick,
  Loading,
  CircleCheck,
  Promotion,
  Delete,
  Setting,
  EditPen,
  View,
  Cpu,
  Connection,
  Operation,
} from '@element-plus/icons-vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { UploadFile, UploadFiles } from 'element-plus'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { relaunch } from '@tauri-apps/plugin-process'
import { check } from '@tauri-apps/plugin-updater'
import { useI18n } from 'vue-i18n'
import { useDark, useDebounceFn, useStorage } from '@vueuse/core'
import { VueMonacoEditor } from '@guolao/vue-monaco-editor'
import { Splitpanes, Pane } from 'splitpanes'
import 'splitpanes/dist/splitpanes.css'
import MarkdownIt from 'markdown-it'
import JSZip from 'jszip'
import ClockThemeToggle from './components/ClockThemeToggle.vue'
import SettingsModal from './components/SettingsModal.vue'

const md = new MarkdownIt({
  html: true,
  linkify: true,
  typographer: true,
})

const { locale, t } = useI18n({ useScope: 'global' })
const isDark = useDark()

type FileType = 'pdf' | 'xlsx' | 'docx'
type FileStatus = 'done' | 'processing'
type ExportFormat = 'markdown' | 'html' | 'json'

interface HistoryFile {
  id: number
  name: string
  type: FileType
  status: FileStatus
  pinned?: boolean
}

interface ContextMenuState {
  visible: boolean
  left: number
  top: number
  fileId: number | null
}

interface ChatMessage {
  id: number
  role: 'ai' | 'user'
  content: string
}

interface DocumentParseResult {
  markdown: string
  confidence?: number | null
  meta?: Record<string, unknown> | null
  tempFilePath?: string | null
}

interface ParseProgressPayload {
  id: string
  percent: number
  stage: string
}

interface ParseProgressState {
  percent: number
  stage: string
}

interface AiRepairDecision {
  need_fix: boolean
  header_row_index?: number
  skip_rows?: unknown[]
  reason?: string
}

interface AppSettingsState {
  provider: string
  apiKeys: Record<string, string>
  baseUrl: string
  selectedModel: string
  customModel: string
  webhookUrl: string
  autoPush: boolean
  strictAlign: boolean
  enableOcr: boolean
  specTolerance: boolean
  flattenHeaders: boolean
  language: string
  fontSize: number
  fontFamily: string
  autoSave: boolean
}

const HISTORY_STORAGE_KEY = 'markdown_gui_history'
const CONTENTS_STORAGE_KEY = 'markdown_gui_contents'
const CHATS_STORAGE_KEY = 'markdown_gui_chats'

const historyFiles = ref<HistoryFile[]>([])

const selectedId = ref<number | null>(null)
const dialogVisible = ref(false)
const isEditMode = ref(false)
const showSidebarSettingsMenu = ref(false)
const showSettings = ref(false)
const settingsInitialTab = ref('ai')
const AI_PROVIDER_BASE_URLS: Record<string, string> = {
  deepseek: 'https://api.deepseek.com',
  openai: 'https://api.openai.com',
  gemini: 'https://generativelanguage.googleapis.com/v1beta',
  siliconflow: 'https://api.siliconflow.cn',
}
const DEFAULT_AI_PROVIDER = 'deepseek'
const DEFAULT_AI_BASE_URL = AI_PROVIDER_BASE_URLS[DEFAULT_AI_PROVIDER]
const DEFAULT_AI_API_KEYS: Record<string, string> = {
  deepseek: '',
  openai: '',
  gemini: '',
  siliconflow: '',
  custom: '',
}
const settings = useStorage<AppSettingsState>('app-settings', {
  provider: DEFAULT_AI_PROVIDER,
  apiKeys: { ...DEFAULT_AI_API_KEYS },
  baseUrl: DEFAULT_AI_BASE_URL,
  selectedModel: '',
  customModel: '',
  webhookUrl: '',
  autoPush: false,
  strictAlign: true,
  enableOcr: false,
  specTolerance: false,
  flattenHeaders: false,
  language: 'zh-CN',
  fontSize: 14,
  fontFamily: 'Consolas, monospace',
  autoSave: true,
}, localStorage, {
  mergeDefaults: true,
})
const DEFAULT_EDITOR_FONT_FAMILY = 'Consolas, monospace'
const supportedEditorFontFamilies = new Set([
  DEFAULT_EDITOR_FONT_FAMILY,
  '"Fira Code", monospace',
  '"JetBrains Mono", monospace',
  '"Cascadia Code", monospace',
  '"Source Code Pro", monospace',
  'Menlo, Monaco, monospace',
])
const normalizeEditorFontFamily = (fontFamily: string): string =>
  supportedEditorFontFamilies.has(fontFamily) ? fontFamily : DEFAULT_EDITOR_FONT_FAMILY
const bundledEditorFontFamilies = ['Fira Code', 'JetBrains Mono', 'Cascadia Code', 'Source Code Pro']
const preloadBundledEditorFonts = () => {
  if (!document.fonts?.load) return
  bundledEditorFontFamilies.forEach((family) => {
    document.fonts.load(`14px "${family}"`).catch(() => {})
  })
}

const normalizeAiApiKeys = (apiKeys: unknown): Record<string, string> => {
  const normalized = { ...DEFAULT_AI_API_KEYS }

  if (apiKeys && typeof apiKeys === 'object' && !Array.isArray(apiKeys)) {
    Object.entries(apiKeys as Record<string, unknown>).forEach(([provider, key]) => {
      normalized[provider] = typeof key === 'string' ? key : ''
    })
  }

  return normalized
}

const getCurrentAiApiKey = (): string =>
  settings.value.apiKeys?.[settings.value.provider]?.trim() ?? ''

const migrateAiSettings = () => {
  const legacySettings = settings.value as AppSettingsState & {
    apiKey?: string
    aiKey?: string
    aiBaseUrl?: string
  }

  if (!settings.value.provider) {
    settings.value.provider = DEFAULT_AI_PROVIDER
  }

  settings.value.apiKeys = normalizeAiApiKeys(settings.value.apiKeys)
  const apiKeyProvider = settings.value.provider || DEFAULT_AI_PROVIDER
  const legacyApiKey = [
    legacySettings.apiKey,
    legacySettings.aiKey,
  ].find((key): key is string => typeof key === 'string' && key.trim() !== '')

  if (legacyApiKey && !settings.value.apiKeys[apiKeyProvider]?.trim()) {
    settings.value.apiKeys[apiKeyProvider] = legacyApiKey
  }

  const legacyDeepSeekApiKey = localStorage.getItem('deepseek_api_key') ?? ''
  if (legacyDeepSeekApiKey.trim() && !settings.value.apiKeys.deepseek?.trim()) {
    settings.value.apiKeys.deepseek = legacyDeepSeekApiKey
  }

  if (legacySettings.aiBaseUrl && (!settings.value.baseUrl || settings.value.baseUrl === DEFAULT_AI_BASE_URL)) {
    settings.value.baseUrl = legacySettings.aiBaseUrl
  } else if (!settings.value.baseUrl) {
    settings.value.baseUrl = DEFAULT_AI_BASE_URL
  }

  if (!settings.value.selectedModel) {
    settings.value.selectedModel = ''
  }

  if (!settings.value.customModel) {
    settings.value.customModel = ''
  }

  delete legacySettings.apiKey
  delete legacySettings.aiKey
  delete legacySettings.aiBaseUrl
}

const safeMigrateAiSettings = () => {
  try {
    migrateAiSettings()
  } catch (error) {
    console.error('AI settings migration failed:', error)
  }
}

safeMigrateAiSettings()
const selectedLang = ref(String(locale.value))
const markdownContents = ref<Record<number, string>>({})
const parseProgressByFile = ref<Record<number, ParseProgressState>>({})
const chatHistories = ref<Record<number, ChatMessage[]>>({})
const deletedFileIds = new Set<number>()
const isBatchExportMode = ref(false)
const selectedBatchFileIds = ref<number[]>([])
const batchExportFormat = ref<ExportFormat>('markdown')
const fileListRef = ref<HTMLElement | null>(null)
const contextMenu = ref<ContextMenuState>({
  visible: false,
  left: 0,
  top: 0,
  fileId: null,
})
const sliderStyle = ref<Record<string, string>>({
  top: '0px',
  height: '0px',
  opacity: '0',
})
let unlistenParseProgress: (() => void) | null = null

const updateSlider = async (fileId: number) => {
  await nextTick()
  const listEl = fileListRef.value
  if (!listEl) return
  const item = listEl.querySelector(`[data-file-id="${fileId}"]`) as HTMLElement | null
  if (!item) return

  const newTop = item.offsetTop
  const newHeight = item.offsetHeight
  const oldTop = parseFloat(sliderStyle.value.top || '0')
  const oldHeight = parseFloat(sliderStyle.value.height || '0')

  if (oldHeight === 0 || sliderStyle.value.opacity === '0') {
    sliderStyle.value = {
      top: `${newTop}px`,
      height: `${newHeight}px`,
      opacity: '1',
      transition: 'top 0.35s cubic-bezier(0.34, 1.56, 0.64, 1), height 0.35s cubic-bezier(0.34, 1.56, 0.64, 1), opacity 0.25s ease',
    }
    return
  }

  const movingDown = newTop > oldTop
  const stretchTop = movingDown ? oldTop : newTop
  const stretchHeight = movingDown
    ? newTop + newHeight - oldTop
    : oldTop + oldHeight - newTop

  sliderStyle.value = {
    top: `${stretchTop}px`,
    height: `${stretchHeight}px`,
    opacity: '1',
    transition: 'top 0.26s cubic-bezier(0.4, 0, 0.2, 1), height 0.26s cubic-bezier(0.4, 0, 0.2, 1)',
  }

  setTimeout(() => {
    sliderStyle.value = {
      top: `${newTop}px`,
      height: `${newHeight}px`,
      opacity: '1',
      transition: 'top 0.4s cubic-bezier(0.34, 1.56, 0.64, 1), height 0.4s cubic-bezier(0.34, 1.56, 0.64, 1)',
    }
  }, 240)
}

const currentMarkdown = computed(() => {
  if (selectedId.value === null) return null
  return markdownContents.value[selectedId.value] ?? null
})

const activeMarkdown = computed({
  get: () => {
    return selectedId.value !== null ? (markdownContents.value[selectedId.value] || '') : ''
  },
  set: (val: string) => {
    if (selectedId.value !== null) {
      markdownContents.value[selectedId.value] = val
    }
  },
})

const saveToLocal = useDebounceFn((content: string) => {
  if (settings.value.autoSave && selectedId.value !== null) {
    markdownContents.value[selectedId.value] = content
    localStorage.setItem(CONTENTS_STORAGE_KEY, JSON.stringify(markdownContents.value))
    console.log(t('app.autoSaved'))
  }
}, 1000)

const monacoEditorKey = computed(() =>
  `${settings.value.fontFamily}-${settings.value.fontSize}-${isDark.value ? 'dark' : 'light'}`,
)

const monacoEditorOptions = computed(() => ({
  minimap: { enabled: false },
  wordWrap: 'on',
  lineNumbers: 'on',
  renderLineHighlight: 'all',
  scrollBeyondLastLine: false,
  smoothScrolling: true,
  padding: { top: 20 },
  fontSize: settings.value.fontSize,
  fontFamily: settings.value.fontFamily,
  automaticLayout: true,
}))

const currentSelectedFile = computed(() => historyFiles.value.find((file) => file.id === selectedId.value) ?? null)

const normalizeProgressPercent = (percent: number): number => {
  if (!Number.isFinite(percent)) return 10
  return Math.max(0, Math.min(100, Math.round(percent)))
}

const getDefaultParseProgress = (): ParseProgressState => ({
  percent: 10,
  stage: '读取文件',
})

const currentParseProgress = computed(() => {
  const fileId = currentSelectedFile.value?.id
  if (fileId === undefined) return getDefaultParseProgress()
  return parseProgressByFile.value[fileId] ?? getDefaultParseProgress()
})

const currentParseProgressStyle = computed(() => ({
  width: `${currentParseProgress.value.percent}%`,
}))

const getLocalizedParseStage = (stage: string): string => {
  const normalizedStage = stage.trim()
  if (normalizedStage === '收到请求' || normalizedStage === '读取文件') {
    return t('app.stageReading')
  }
  if (normalizedStage === '清洗数据') {
    return t('app.stageCleaning')
  }
  if (normalizedStage === '生成 Markdown') {
    return t('app.stageGenerating')
  }
  return normalizedStage || t('app.stageReading')
}

const updateParseProgress = (fileId: number, percent: number, stage: string) => {
  if (!historyFiles.value.some((file) => file.id === fileId)) return
  parseProgressByFile.value = {
    ...parseProgressByFile.value,
    [fileId]: {
      percent: normalizeProgressPercent(percent),
      stage: stage || '读取文件',
    },
  }
}

const clearParseProgress = (fileId: number) => {
  if (!(fileId in parseProgressByFile.value)) return
  const { [fileId]: _removed, ...nextProgress } = parseProgressByFile.value
  parseProgressByFile.value = nextProgress
}

const handleParseProgress = (payload: ParseProgressPayload) => {
  const fileId = Number(payload.id)
  if (!Number.isFinite(fileId)) return
  updateParseProgress(fileId, payload.percent, payload.stage)
}

const currentMessages = computed(() =>
  selectedId.value !== null ? (chatHistories.value[selectedId.value] || []) : [],
)

const normalizeMarkdownText = (text: string): string =>
  text
    .replace(/\\r\\n/g, '\n')
    .replace(/\\n/g, '\n')
    .replace(/\r\n/g, '\n')

const splitMarkdownRow = (line: string): string[] =>
  line
    .trim()
    .replace(/^\|/, '')
    .replace(/\|$/, '')
    .split('|')
    .map((cell) => cell.trim())

const isMarkdownSeparatorRow = (line: string): boolean => {
  const cells = splitMarkdownRow(line)
  return cells.length > 0 && cells.every((cell) => /^:?-{3,}:?$/.test(cell.replace(/\s/g, '')))
}

const normalizeHeaderName = (header: string, index: number, usedHeaders: Record<string, number>): string => {
  const unnamedMatch = header.match(/^Unnamed:\s*(\d+)$/i)
  const baseName = unnamedMatch ? `未知列_${unnamedMatch[1]}` : header.trim() || `列_${index + 1}`
  const count = usedHeaders[baseName] ?? 0
  usedHeaders[baseName] = count + 1
  return count === 0 ? baseName : `${baseName}_${count + 1}`
}

const isInvalidMarkdownHeader = (header: string): boolean => {
  const value = header.trim()
  return !value || /^NaN$/i.test(value) || /Unnamed:/i.test(value) || /^未知列(?:_\d+)?$/i.test(value) || /^列_\d+$/i.test(value)
}

const cleanTableCell = (cell: string): string => {
  const value = cell.trim()
  return /^NaN$/i.test(value) ? '' : value
}

const parseMarkdownTableToJson = (markdown: string): Record<string, string>[] => {
  const tableLines = normalizeMarkdownText(markdown)
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.startsWith('|') && line.endsWith('|'))

  if (tableLines.length < 2) return []

  const headerLineIndex = tableLines.findIndex((_line, index) => {
    const nextLine = tableLines[index + 1]
    return Boolean(nextLine) && isMarkdownSeparatorRow(nextLine)
  })
  const headerLine = tableLines[headerLineIndex >= 0 ? headerLineIndex : 0]
  const dataStartIndex = headerLineIndex >= 0 ? headerLineIndex + 2 : 2
  const usedHeaders: Record<string, number> = {}
  const headers = splitMarkdownRow(headerLine).map((header, index) =>
    normalizeHeaderName(header, index, usedHeaders),
  )

  return tableLines
    .slice(dataStartIndex)
    .filter((line) => !isMarkdownSeparatorRow(line))
    .map((line) => {
      const cells = splitMarkdownRow(line)
      return headers.reduce<Record<string, string>>((row, header, index) => {
        if (isInvalidMarkdownHeader(header)) return row
        row[header] = cleanTableCell(cells[index] ?? '')
        return row
      }, {})
    })
    .filter((row) => Object.values(row).some((value) => value !== ''))
}

const formatLocalIsoDateTime = (date: Date): string => {
  const pad = (value: number): string => String(value).padStart(2, '0')
  return [
    date.getFullYear(),
    pad(date.getMonth() + 1),
    pad(date.getDate()),
  ].join('-') + `T${[
    pad(date.getHours()),
    pad(date.getMinutes()),
    pad(date.getSeconds()),
  ].join(':')}`
}

const pushParsedResultToWebhook = async (fileName: string, markdown: string) => {
  const webhookUrl = settings.value.webhookUrl.trim()
  if (!webhookUrl || !settings.value.autoPush) return

  try {
    const response = await fetch(webhookUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        file_name: fileName,
        parsed_at: formatLocalIsoDateTime(new Date()),
        markdown,
        json_data: parseMarkdownTableToJson(markdown),
        source: 'MarkdownGUI',
      }),
    })

    if (!response.ok) {
      throw new Error(`Webhook responded with ${response.status}`)
    }
  } catch {
    ElMessage.warning(t('app.webhookFailed'))
  }
}

const jsonOutput = computed(() => {
  if (currentMarkdown.value) {
    const parsedRows = parseMarkdownTableToJson(currentMarkdown.value)
    if (parsedRows.length > 0) {
      return JSON.stringify(parsedRows, null, 2)
    }
  }

  return ''
})

const selectedFile = computed(() => {
  if (selectedId.value === null) return null
  return historyFiles.value.find((file) => file.id === selectedId.value) ?? null
})

const contextMenuFile = computed(() => {
  if (contextMenu.value.fileId === null) return null
  return historyFiles.value.find((file) => file.id === contextMenu.value.fileId) ?? null
})

const renderedHtml = computed(() => {
  if (currentMarkdown.value === null) return ''
  return md.render(cleanMarkdownTable(currentMarkdown.value))
})

const fileToBase64 = (file: File): Promise<string> => {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const arrayBuffer = reader.result as ArrayBuffer
      const bytes = new Uint8Array(arrayBuffer)
      let binary = ''
      for (let i = 0; i < bytes.length; i++) {
        binary += String.fromCharCode(bytes[i])
      }
      resolve(btoa(binary))
    }
    reader.onerror = () => reject(new Error(t('app.fileReadFailed')))
    reader.readAsArrayBuffer(file)
  })
}

const hasTauriIpcBridge = (): boolean =>
  typeof window !== 'undefined'
  && (
    typeof (window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__ !== 'undefined'
    || typeof (window as Window & { __TAURI__?: unknown }).__TAURI__ !== 'undefined'
    || typeof (window as Window & { __TAURI_IPC__?: unknown }).__TAURI_IPC__ === 'function'
  )

const invokeTauriCommand = async <T,>(command: string, args?: Record<string, unknown>): Promise<T> => {
  if (!hasTauriIpcBridge()) {
    throw new Error(t('app.useDesktopApp'))
  }

  return invoke<T>(command, args)
}

const normalizeParseResult = (result: DocumentParseResult | string): DocumentParseResult =>
  typeof result === 'string' ? { markdown: result } : result

const cleanupTempParseFile = async (tempFilePath?: string | null) => {
  if (!tempFilePath) return

  try {
    await invokeTauriCommand<void>('cleanup_temp_document', {
      filePath: tempFilePath,
    })
  } catch {
    // Best-effort cleanup only; parsing results should not be affected.
  }
}

const cleanMarkdownTable = (text: string): string => {
  const fixedText = normalizeMarkdownText(text)
    .replace(/\|\s*NaN\s*(?=\|)/g, '| ')
    .replace(/Unnamed:\s*\d+/g, '')

  const lines = fixedText.split('\n')
  const result: string[] = []

  lines.forEach((line, index) => {
    const currentLine = line.trim()
    const previousLine = lines[index - 1]?.trim() ?? ''
    const nextLine = lines[index + 1]?.trim() ?? ''
    const isTableLine = currentLine.startsWith('|') && currentLine.endsWith('|')
    const isPreviousTableLine = previousLine.startsWith('|') && previousLine.endsWith('|')
    const isNextTableLine = nextLine.startsWith('|') && nextLine.endsWith('|')

    if (isTableLine && !isPreviousTableLine && result[result.length - 1] !== '') {
      result.push('')
    }

    result.push(line)

    if (isTableLine && !isNextTableLine) {
      result.push('')
    }
  })

  return result.join('\n').replace(/\n{3,}/g, '\n\n')
}

const typeIconColor: Record<FileType, string> = {
  pdf: '#f56c6c',
  xlsx: '#67c23a',
  docx: '#409eff',
}

const selectFile = (file: HistoryFile) => {
  selectedId.value = file.id
  updateSlider(file.id)
}

const detectType = (name: string): FileType => {
  const lower = name.toLowerCase()
  if (lower.endsWith('.xlsx') || lower.endsWith('.xls')) return 'xlsx'
  if (lower.endsWith('.docx') || lower.endsWith('.doc')) return 'docx'
  return 'pdf'
}

const updateFileStatus = (fileId: number, status: FileStatus) => {
  const index = historyFiles.value.findIndex((item) => item.id === fileId)
  if (index === -1) return
  historyFiles.value[index] = {
    ...historyFiles.value[index],
    status,
  }
}

const closeContextMenu = () => {
  contextMenu.value = {
    ...contextMenu.value,
    visible: false,
    fileId: null,
  }
}

const handleFileItemClick = (file: HistoryFile) => {
  if (isBatchExportMode.value) {
    toggleBatchFileSelection(file, !selectedBatchFileIds.value.includes(file.id))
    return
  }
  selectFile(file)
}

const handleFileItemContextMenu = (file: HistoryFile, event: MouseEvent) => {
  if (isBatchExportMode.value) return
  openFileContextMenu(file, event)
}

const enterBatchExportMode = () => {
  closeContextMenu()
  isBatchExportMode.value = true
  selectedBatchFileIds.value = []
  batchExportFormat.value = 'markdown'
}

const exitBatchExportMode = () => {
  isBatchExportMode.value = false
  selectedBatchFileIds.value = []
  batchExportFormat.value = 'markdown'
}

const handleMainAreaClick = () => {
  if (isBatchExportMode.value) {
    exitBatchExportMode()
  }
}

const toggleBatchFileSelection = (file: HistoryFile, checked: boolean) => {
  if (file.status !== 'done') return

  const selectedIds = new Set(selectedBatchFileIds.value)
  if (checked) {
    selectedIds.add(file.id)
  } else {
    selectedIds.delete(file.id)
  }
  selectedBatchFileIds.value = Array.from(selectedIds)
}

const handleBatchCheckboxChange = (file: HistoryFile, checked: string | number | boolean) => {
  toggleBatchFileSelection(file, Boolean(checked))
}

const selectedBatchCount = computed(() => selectedBatchFileIds.value.length)

const handleGlobalPointerDown = (event: PointerEvent) => {
  const target = event.target
  if (!(target instanceof Element)) return

  if (contextMenu.value.visible && !target.closest('.file-context-menu')) {
    closeContextMenu()
  }

  if (showSidebarSettingsMenu.value && !target.closest('.sidebar-footer')) {
    showSidebarSettingsMenu.value = false
  }
}

const handleGlobalContextMenu = (event: MouseEvent) => {
  if (!contextMenu.value.visible) return
  const target = event.target
  if (target instanceof Element && target.closest('.file-item')) return
  closeContextMenu()
}

const openFileContextMenu = (file: HistoryFile, event: MouseEvent) => {
  selectedId.value = file.id
  updateSlider(file.id)
  contextMenu.value = {
    visible: true,
    left: Math.min(event.clientX, window.innerWidth - 168),
    top: Math.min(event.clientY, window.innerHeight - 136),
    fileId: file.id,
  }
}

const sortHistoryFiles = () => {
  historyFiles.value = [...historyFiles.value].sort((a, b) => Number(b.pinned) - Number(a.pinned))
}

const togglePinFile = (id: number) => {
  closeContextMenu()
  const index = historyFiles.value.findIndex((file) => file.id === id)
  if (index === -1) return
  const target = {
    ...historyFiles.value[index],
    pinned: !historyFiles.value[index].pinned,
  }
  const rest = historyFiles.value.filter((file) => file.id !== id)
  historyFiles.value = target.pinned ? [target, ...rest] : [...rest, target]
  sortHistoryFiles()
  updateSlider(id)
}

const renameFile = async (id: number) => {
  const file = historyFiles.value.find((item) => item.id === id)
  if (!file) return
  closeContextMenu()

  try {
    const { value } = await ElMessageBox.prompt(t('app.renamePlaceholder'), t('app.renameTitle'), {
      inputValue: file.name,
      confirmButtonText: t('app.save'),
      cancelButtonText: t('app.cancel'),
      inputPattern: /\S+/,
      inputErrorMessage: t('app.fileNameEmpty'),
    })
    const nextName = value.trim()
    const index = historyFiles.value.findIndex((item) => item.id === id)
    if (!nextName || index === -1) return
    historyFiles.value[index] = {
      ...historyFiles.value[index],
      name: nextName,
      type: detectType(nextName),
    }
    ElMessage.success(t('app.fileRenamed'))
  } catch {
    // User cancelled the rename dialog.
  }
}

const deleteFile = (id: number) => {
  closeContextMenu()
  deletedFileIds.add(id)
  clearParseProgress(id)
  historyFiles.value = historyFiles.value.filter((file) => file.id !== id)
  selectedBatchFileIds.value = selectedBatchFileIds.value.filter((fileId) => fileId !== id)

  const nextContents = { ...markdownContents.value }
  delete nextContents[id]
  markdownContents.value = nextContents

  const nextChats = { ...chatHistories.value }
  delete nextChats[id]
  chatHistories.value = nextChats

  if (selectedId.value === id) {
    selectedId.value = null
    sliderStyle.value = {
      top: '0px',
      height: '0px',
      opacity: '0',
    }
  }
}

const clearAllHistory = async () => {
  try {
    await ElMessageBox.confirm(
      t('app.clearHistoryConfirm'),
      t('app.clearHistoryTitle'),
      {
        confirmButtonText: t('app.clear'),
        cancelButtonText: t('app.cancel'),
        customClass: 'modern-confirm-dialog',
        confirmButtonClass: 'modern-danger-btn',
        cancelButtonClass: 'modern-cancel-btn',
        center: true,
        closeOnClickModal: true,
        roundButton: false,
      },
    )

    historyFiles.value.forEach((file) => deletedFileIds.add(file.id))
    historyFiles.value = []
    markdownContents.value = {}
    parseProgressByFile.value = {}
    chatHistories.value = {}
    exitBatchExportMode()
    selectedId.value = null
    sliderStyle.value = {
      top: '0px',
      height: '0px',
      opacity: '0',
    }
    ElMessage.success(t('app.historyCleaned'))
  } catch {
    // User cancelled the clear action.
  }
}

const loadPersistedState = () => {
  try {
    const storedContents = localStorage.getItem(CONTENTS_STORAGE_KEY)
    if (storedContents) {
      const parsedContents = JSON.parse(storedContents) as unknown
      if (parsedContents && typeof parsedContents === 'object' && !Array.isArray(parsedContents)) {
        markdownContents.value = Object.entries(parsedContents).reduce<Record<number, string>>(
          (contents, [id, value]) => {
            if (typeof value === 'string') {
              contents[Number(id)] = value
            }
            return contents
          },
          {},
        )
      }
    }

    const storedChats = localStorage.getItem(CHATS_STORAGE_KEY)
    if (storedChats) {
      const parsedChats = JSON.parse(storedChats) as unknown
      if (parsedChats && typeof parsedChats === 'object' && !Array.isArray(parsedChats)) {
        chatHistories.value = Object.entries(parsedChats).reduce<Record<number, ChatMessage[]>>(
          (histories, [id, value]) => {
            if (Array.isArray(value)) {
              histories[Number(id)] = value
                .filter((message): message is ChatMessage =>
                  Boolean(message) &&
                  typeof message === 'object' &&
                  typeof (message as ChatMessage).id === 'number' &&
                  ((message as ChatMessage).role === 'ai' || (message as ChatMessage).role === 'user') &&
                  typeof (message as ChatMessage).content === 'string',
                )
                .map((message) => ({
                  id: message.id,
                  role: message.role,
                  content: message.content,
                }))
            }
            return histories
          },
          {},
        )
      }
    }

    const storedHistory = localStorage.getItem(HISTORY_STORAGE_KEY)
    if (storedHistory) {
      const parsedHistory = JSON.parse(storedHistory) as unknown
      if (Array.isArray(parsedHistory)) {
        historyFiles.value = parsedHistory
          .filter((item): item is Partial<HistoryFile> => Boolean(item) && typeof item === 'object')
          .map((item): HistoryFile => ({
            id: Number(item.id),
            name: typeof item.name === 'string' ? item.name : t('app.unnamedFile'),
            type: typeof item.name === 'string' ? detectType(item.name) : 'pdf',
            status: 'done',
            pinned: Boolean(item.pinned),
          }))
          .filter((item) => Number.isFinite(item.id))
        sortHistoryFiles()
      }
    }

    selectedId.value = historyFiles.value[0]?.id ?? null
    if (selectedId.value !== null) {
      updateSlider(selectedId.value)
    }
  } catch {
    localStorage.removeItem(HISTORY_STORAGE_KEY)
    localStorage.removeItem(CONTENTS_STORAGE_KEY)
    localStorage.removeItem(CHATS_STORAGE_KEY)
    historyFiles.value = []
    markdownContents.value = {}
    chatHistories.value = {}
    selectedId.value = null
  }
}

const loadAiSettings = () => {
  safeMigrateAiSettings()
  selectedLang.value = settings.value.language || String(locale.value)
  locale.value = selectedLang.value
  if (!getCurrentAiApiKey()) {
    showSettings.value = true
  }
}

const openSettingDialog = () => {
  openSetting('ai')
}

const openSetting = (tab = 'ai') => {
  settingsInitialTab.value = tab
  selectedLang.value = String(locale.value)
  showSettings.value = true
}

const updateInfo = ref<{
  version: string
  update: any
} | null>(null)
const updateProgress = ref(0)
const updateState = ref<'idle' | 'available' | 'downloading' | 'installing'>('idle')

const checkForUpdates = async () => {
  try {
    const update = await check()
    if (!update) return

    updateInfo.value = { version: update.version, update }
    updateState.value = 'available'

    try {
      await ElMessageBox.confirm(
        t('app.updateContent') + ' ' + update.version,
        t('app.updateTitle'),
        {
          confirmButtonText: t('app.updateConfirm'),
          cancelButtonText: t('app.updateCancel'),
          type: 'info',
        },
      )
      await startUpdateDownload()
    } catch {
      // User cancelled — badge button remains visible for manual trigger
    }
  } catch {
    // Update checks should never interrupt normal app usage.
  }
}

const startUpdateDownload = async () => {
  if (!updateInfo.value || updateState.value !== 'available') return

  try {
    updateState.value = 'downloading'
    updateProgress.value = 0
    ElMessage.info(t('app.updateDownloadingMsg'))

    await updateInfo.value.update.download((event: { event: string; data: { contentLength?: number; totalDownloaded: number } }) => {
      if (event.event === 'Progress') {
        const total = event.data.contentLength || 0
        if (total > 0) {
          updateProgress.value = Math.round(
            (event.data.totalDownloaded / total) * 100,
          )
        }
      }
    })

    updateState.value = 'installing'
    ElMessage.success(t('app.updateInstallingMsg'))
    await updateInfo.value.update.install()
    await relaunch()
  } catch {
    updateState.value = 'available'
    ElMessage.error(t('app.updateFailed'))
  }
}

onMounted(() => {
  preloadBundledEditorFonts()
  loadPersistedState()
  loadAiSettings()
  listen<ParseProgressPayload>('parse-progress', (event) => {
    handleParseProgress(event.payload)
  })
    .then((unlisten) => {
      unlistenParseProgress = unlisten
    })
    .catch(() => {})
  window.setTimeout(() => {
    void checkForUpdates()
  }, 5000)
  document.addEventListener('pointerdown', handleGlobalPointerDown, true)
  document.addEventListener('contextmenu', handleGlobalContextMenu, true)
})

onBeforeUnmount(() => {
  unlistenParseProgress?.()
  unlistenParseProgress = null
  document.removeEventListener('pointerdown', handleGlobalPointerDown, true)
  document.removeEventListener('contextmenu', handleGlobalContextMenu, true)
})

watch(
  historyFiles,
  (files) => {
    localStorage.setItem(HISTORY_STORAGE_KEY, JSON.stringify(files))
  },
  { deep: true },
)

watch(
  markdownContents,
  (contents) => {
    if (settings.value.autoSave) {
      localStorage.setItem(CONTENTS_STORAGE_KEY, JSON.stringify(contents))
    }
  },
  { deep: true },
)

watch(activeMarkdown, (newVal) => {
  saveToLocal(newVal)
})

watch(
  () => settings.value.language,
  (lang) => {
    selectedLang.value = lang
    locale.value = lang
  },
  { immediate: true },
)

watch(
  () => settings.value.apiKeys?.deepseek,
  (key) => {
    localStorage.setItem('deepseek_api_key', (key ?? '').trim())
  },
  { immediate: true },
)

watch(
  () => settings.value.fontFamily,
  (font) => {
    const normalizedFont = normalizeEditorFontFamily(font)
    if (normalizedFont !== font) {
      settings.value.fontFamily = normalizedFont
    }
    document.documentElement.style.setProperty('--editor-font-family', normalizedFont)
  },
  { immediate: true },
)

watch(
  () => settings.value.fontSize,
  (size) => {
    document.documentElement.style.setProperty('--editor-font-size', `${size}px`)
  },
  { immediate: true },
)

watch(
  chatHistories,
  (histories) => {
    localStorage.setItem(CHATS_STORAGE_KEY, JSON.stringify(histories))
  },
  { deep: true },
)

watch(showSidebarSettingsMenu, (visible) => {
  if (visible) {
    selectedLang.value = String(locale.value)
  }
})

const handleChange = async (file: UploadFile, _files: UploadFiles) => {
  const newFile: HistoryFile = {
    id: Date.now(),
    name: file.name,
    type: detectType(file.name),
    status: 'processing',
  }
  historyFiles.value.unshift(newFile)
  sortHistoryFiles()

  markdownContents.value[newFile.id] = ''
  updateParseProgress(newFile.id, 10, '读取文件')
  selectedId.value = newFile.id
  updateSlider(newFile.id)

  try {
    const rawFile = file.raw
    if (!rawFile) {
      throw new Error(t('app.fileReadFailed'))
    }
    const base64Data = await fileToBase64(rawFile)
    const rawResult = await invokeTauriCommand<DocumentParseResult | string>('convert_document_bytes', {
      fileName: file.name,
      fileDataBase64: base64Data,
      strictAlign: settings.value.strictAlign,
      enableOcr: settings.value.enableOcr,
      specTolerance: settings.value.specTolerance,
      flattenHeaders: settings.value.flattenHeaders,
      requestId: String(newFile.id),
    })
    let parseResult = normalizeParseResult(rawResult)
    const tempFilePath = parseResult.tempFilePath
    try {
      if ((parseResult.confidence ?? 1) < 0.7 && parseResult.meta) {
        parseResult = await repairWithAI(newFile.id, parseResult)
      }
    } finally {
      await cleanupTempParseFile(tempFilePath)
    }
    if (deletedFileIds.has(newFile.id)) return
    const markdown = parseResult.markdown
    markdownContents.value[newFile.id] = markdown
    selectedId.value = newFile.id
    updateSlider(newFile.id)
    updateParseProgress(newFile.id, 100, '生成 Markdown')
    updateFileStatus(newFile.id, 'done')
    clearParseProgress(newFile.id)
    void pushParsedResultToWebhook(newFile.name, markdown)
    ElMessage.success(`「${newFile.name}」${t('app.parseComplete')}`)
  } catch (error) {
    if (deletedFileIds.has(newFile.id)) return
    updateFileStatus(newFile.id, 'done')
    clearParseProgress(newFile.id)
    const message = error instanceof Error ? error.message : String(error)
    ElMessage.error(`${t('app.parseFailed')}: ${message}`)
  }
}

const openDataDialog = () => {
  dialogVisible.value = true
}

const exportFormatExtensions: Record<ExportFormat, string> = {
  markdown: 'md',
  html: 'html',
  json: 'json',
}

const exportFormatMimeTypes: Record<ExportFormat, string> = {
  markdown: 'text/markdown;charset=utf-8;',
  html: 'text/html;charset=utf-8;',
  json: 'application/json;charset=utf-8;',
}

const escapeHtmlText = (text: string): string =>
  text.replace(/[&<>"']/g, (char) => ({
    '&': '&amp;',
    '<': '&lt;',
    '>': '&gt;',
    '"': '&quot;',
    "'": '&#39;',
  }[char] ?? char))

const getExportBaseFileName = (file?: HistoryFile | null): string => {
  const sourceName = file?.name ?? 'document'
  return sourceName.replace(/\.[^/.]+$/, '') || 'document'
}

const renderMarkdownContent = (markdown: string): string =>
  md.render(cleanMarkdownTable(markdown))

const buildHtmlDocument = (fileName: string, markdown: string): string => `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>${escapeHtmlText(fileName)}</title>
  <style>
    body { margin: 24px; color: #2c3550; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
    .markdown-body { width: 100%; }
    table { border-collapse: collapse; width: 100%; }
    th, td { border: 1px solid #ddd; padding: 8px; }
    th { background: #f6f8fc; text-align: left; }
  </style>
</head>
<body>
  <div class="markdown-body">${renderMarkdownContent(markdown)}</div>
</body>
</html>`

const buildExportArtifact = (file: HistoryFile, format: ExportFormat) => {
  const markdown = markdownContents.value[file.id] ?? ''
  const baseName = getExportBaseFileName(file)
  const extension = exportFormatExtensions[format]
  const content = {
    markdown,
    html: buildHtmlDocument(baseName, markdown),
    json: JSON.stringify(parseMarkdownTableToJson(markdown), null, 2),
  }[format]

  return {
    content,
    type: exportFormatMimeTypes[format],
    fileName: `${baseName}.${extension}`,
  }
}

const triggerBlobDownload = (blob: Blob, fileName: string) => {
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')

  link.href = url
  link.download = fileName
  link.style.display = 'none'
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  URL.revokeObjectURL(url)
}

const downloadBlob = (content: string, type: string, fileName: string) => {
  triggerBlobDownload(new Blob([content], { type }), fileName)
}

const exportFile = (file: HistoryFile, format: ExportFormat): boolean => {
  if (!markdownContents.value[file.id]) {
    ElMessage.warning(t('app.noExportData'))
    return false
  }

  const artifact = buildExportArtifact(file, format)
  downloadBlob(artifact.content, artifact.type, artifact.fileName)

  if (format === 'markdown') {
    ElMessage.success(t('app.exportedMarkdown'))
  } else if (format === 'html') {
    ElMessage.success(t('app.exportedHtml'))
  } else {
    ElMessage.success(t('app.exportedJson'))
  }

  return true
}

const handleExport = (command: string) => {
  if (command !== 'markdown' && command !== 'html' && command !== 'json' && command !== 'csv') {
    ElMessage.warning(t('app.noExportData'))
    return
  }

  const file = selectedFile.value
  if (!file || !currentMarkdown.value) {
    ElMessage.warning(t('app.noExportData'))
    return
  }

  if (command === 'csv') {
    exportCSV()
  } else if (command === 'markdown') {
    exportMarkdown()
  } else if (command === 'html') {
    exportHTML()
  } else {
    exportJSON()
  }
}

const handleBatchExport = async () => {
  if (selectedBatchFileIds.value.length === 0) {
    ElMessage.warning(t('app.batchNoSelected'))
    return
  }

  const selectedIds = new Set(selectedBatchFileIds.value)
  const selectedFiles = historyFiles.value.filter((file) => selectedIds.has(file.id) && file.status === 'done')

  if (selectedFiles.length === 0) {
    ElMessage.warning(t('app.batchNoSelected'))
    return
  }

  if (selectedFiles.length === 1) {
    if (exportFile(selectedFiles[0], batchExportFormat.value)) {
      exitBatchExportMode()
    }
    return
  }

  const zip = new JSZip()
  selectedFiles.forEach((file) => {
    const artifact = buildExportArtifact(file, batchExportFormat.value)
    zip.file(artifact.fileName, artifact.content)
  })

  const blob = await zip.generateAsync({ type: 'blob' })
  const timestamp = formatLocalIsoDateTime(new Date())
    .replace(/[-:]/g, '')
    .replace('T', '_')
  triggerBlobDownload(blob, `MarkdownGUI_export_${timestamp}.zip`)
  exitBatchExportMode()
}

const exportMarkdown = () => {
  const file = selectedFile.value
  if (file) exportFile(file, 'markdown')
}

const exportHTML = () => {
  const file = selectedFile.value
  if (file) exportFile(file, 'html')
}

const exportJSON = () => {
  const file = selectedFile.value
  if (file) exportFile(file, 'json')
}

const exportCSV = () => {
  ElMessage.info(t('app.csvComingSoon'))
}

const copyData = async () => {
  try {
    await navigator.clipboard.writeText(jsonOutput.value)
    ElMessage.success(t('app.dataCopied'))
  } catch {
    ElMessage.error(t('app.copyFailed'))
  }
}

const handleWorkbenchClick = () => {
  closeContextMenu()
  if (drawerVisible.value) {
    drawerVisible.value = false
  }
}

const drawerVisible = ref(false)
const inputText = ref('')
const thinking = ref(false)

const initChatIfNeeded = (id: number) => {
  if (chatHistories.value[id]) return

  const currentFile = historyFiles.value.find((file) => file.id === id)
  const fileName = currentFile ? currentFile.name : '当前文档'

  chatHistories.value[id] = [
    {
      id: Date.now(),
      role: 'ai',
      content: `你好！我已经读取了这份《${fileName}》。请问有什么我可以帮您分析或核对的？比如：汇总明细花费，或者提取规格参数。`,
    },
  ]
}

const openAssistant = () => {
  if (selectedId.value !== null) {
    initChatIfNeeded(selectedId.value)
  }
  drawerVisible.value = true
}

watch(selectedId, (newId) => {
  if (drawerVisible.value && newId !== null) {
    initChatIfNeeded(newId)
  }
})

const appendAiMessageContent = (fileId: number, messageId: number, text: string) => {
  if (!text) return
  const messages = chatHistories.value[fileId]
  if (!messages) return
  const index = messages.findIndex((message) => message.id === messageId)
  if (index === -1) return
  messages[index] = {
    ...messages[index],
    content: messages[index].content + text,
  }
  thinking.value = false
}

const setAiMessageContent = (fileId: number, messageId: number, content: string) => {
  const messages = chatHistories.value[fileId]
  if (!messages) return
  const index = messages.findIndex((message) => message.id === messageId)
  if (index === -1) return
  messages[index] = {
    ...messages[index],
    content,
  }
}

const readDeepSeekStream = async (response: Response, fileId: number, messageId: number) => {
  if (!response.body) {
    throw new Error(t('app.requestFailed'))
  }

  const reader = response.body.getReader()
  const decoder = new TextDecoder('utf-8')
  let buffer = ''

  const consumeLine = (line: string) => {
    const trimmedLine = line.trim()
    if (!trimmedLine.startsWith('data:')) return

    const data = trimmedLine.replace(/^data:\s*/, '').trim()
    if (!data || data === '[DONE]') return

    try {
      const chunk = JSON.parse(data) as {
        choices?: Array<{
          delta?: {
            content?: string
          }
        }>
      }
      const content = chunk.choices?.[0]?.delta?.content ?? ''
      appendAiMessageContent(fileId, messageId, content)
    } catch {
      // Ignore malformed heartbeat or partial SSE payload lines.
    }
  }

  while (true) {
    const { done, value } = await reader.read()
    if (done) break

    buffer += decoder.decode(value, { stream: true })
    const lines = buffer.split(/\r?\n/)
    buffer = lines.pop() ?? ''
    lines.forEach(consumeLine)
  }

  buffer += decoder.decode()
  if (buffer.trim()) {
    buffer.split(/\r?\n/).forEach(consumeLine)
  }
}

const getSelectedAiModel = (): string => {
  if (settings.value.selectedModel === 'custom') {
    return settings.value.customModel.trim()
  }
  return settings.value.selectedModel.trim()
}

const getAiChatCompletionsUrl = (): string => {
  const baseUrl = settings.value.baseUrl.trim() || DEFAULT_AI_BASE_URL
  const normalizedBaseUrl = baseUrl.replace(/\/+$/, '')
  return normalizedBaseUrl.endsWith('/chat/completions')
    ? normalizedBaseUrl
    : `${normalizedBaseUrl}/chat/completions`
}

const parseAiRepairDecision = (content: string): AiRepairDecision | null => {
  const fencedMatch = content.match(/```(?:json)?\s*([\s\S]*?)```/i)
  const candidate = (fencedMatch?.[1] ?? content).trim()
  const start = candidate.indexOf('{')
  const end = candidate.lastIndexOf('}')
  if (start === -1 || end === -1 || end <= start) return null

  try {
    const parsed = JSON.parse(candidate.slice(start, end + 1)) as Partial<AiRepairDecision>
    if (typeof parsed.need_fix !== 'boolean') return null
    return parsed as AiRepairDecision
  } catch {
    return null
  }
}

const repairWithAI = async (fileId: number, result: DocumentParseResult): Promise<DocumentParseResult> => {
  void fileId

  try {
    const apiKey = getCurrentAiApiKey()
    const model = getSelectedAiModel()
    if (!apiKey || !model) return result

    const prompt = `以下是一个 Excel 文件的解析结果，置信度较低（${result.confidence ?? 0}）。
解析元信息：${JSON.stringify(result.meta ?? {}, null, 2)}
当前解析出的 Markdown 前 500 字符：${result.markdown.slice(0, 500)}

请判断解析是否正确，如果不正确，请只返回一个 JSON：
{
  "need_fix": true,
  "header_row_index": 3,
  "skip_rows": [0,1,2],
  "reason": "原始表格前3行是标题和空行"
}
如果解析正确，返回：
{ "need_fix": false }
只返回 JSON，不要任何解释。`

    const response = await fetch(getAiChatCompletionsUrl(), {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${apiKey}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model,
        messages: [
          {
            role: 'user',
            content: prompt,
          },
        ],
        temperature: 0,
        stream: false,
      }),
    })

    if (!response.ok) return result

    const payload = await response.json() as {
      choices?: Array<{
        message?: {
          content?: string
        }
      }>
    }
    const content = payload.choices?.[0]?.message?.content
    if (!content) return result

    const decision = parseAiRepairDecision(content)
    if (!decision || !decision.need_fix) return result

    if (typeof decision.header_row_index !== 'number' || !Number.isInteger(decision.header_row_index)) {
      return result
    }

    if (!result.tempFilePath) return result

    const skipRows = Array.isArray(decision.skip_rows)
      ? decision.skip_rows.filter((index): index is number => typeof index === 'number' && Number.isInteger(index) && index >= 0)
      : []

    const repairedResult = await invokeTauriCommand<DocumentParseResult>('repair_document', {
      filePath: result.tempFilePath,
      headerRowIndex: decision.header_row_index,
      skipRows,
    })

    return repairedResult.markdown ? repairedResult : result
  } catch {
    return result
  }
}

const sendMessage = async () => {
  const text = inputText.value.trim()
  if (!text || thinking.value) return

  const fileId = selectedId.value
  if (fileId === null) {
    ElMessage.warning(t('app.aiNoDoc'))
    return
  }

  if (!currentMarkdown.value) {
    ElMessage.warning(t('app.aiNoDoc'))
    return
  }

  const apiKey = getCurrentAiApiKey()
  if (!apiKey) {
    ElMessage.warning(t('app.aiNoKey'))
    openSettingDialog()
    return
  }

  const model = getSelectedAiModel()
  if (!model) {
    ElMessage.warning(t('app.aiNoModel'))
    openSettingDialog()
    return
  }

  initChatIfNeeded(fileId)
  const messages = chatHistories.value[fileId]
  const userMessageId = Date.now()
  const aiMessageId = userMessageId + 1

  messages.push({ id: userMessageId, role: 'user', content: text })
  messages.push({ id: aiMessageId, role: 'ai', content: '' })
  inputText.value = ''
  thinking.value = true

  try {
    const response = await fetch(getAiChatCompletionsUrl(), {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${apiKey}`,
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        model,
        messages: [
          {
            role: 'system',
            content: `你是一个专业的数据分析助手。请基于以下文档内容回答问题：\n${currentMarkdown.value}`,
          },
          {
            role: 'user',
            content: text,
          },
        ],
        stream: true,
      }),
    })

    if (!response.ok) {
      const errorText = await response.text()
      throw new Error(`${t('app.requestFailed')}: ${response.status} ${response.statusText}${errorText ? ` - ${errorText}` : ''}`)
    }

    await readDeepSeekStream(response, fileId, aiMessageId)

    const aiMessage = messages.find((message) => message.id === aiMessageId)
    if (!aiMessage?.content.trim()) {
      setAiMessageContent(fileId, aiMessageId, t('app.requestFailed'))
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    setAiMessageContent(fileId, aiMessageId, `${t('app.requestFailed')}: ${message}`)
    ElMessage.error(message)
  } finally {
    thinking.value = false
  }
}
</script>

<template>
  <el-container class="workbench" @click="handleWorkbenchClick">
    <el-aside width="350px" class="sidebar">
      <div class="sidebar-upload">
        <el-upload
          class="uploader"
          drag
          multiple
          :auto-upload="false"
          :show-file-list="false"
          :on-change="handleChange"
        >
          <div class="upload-inner">
            <el-icon class="upload-icon"><UploadFilled /></el-icon>
            <div class="upload-text">{{ t('app.dropHint') }}</div>
          </div>
        </el-upload>
      </div>

      <div class="sidebar-list">
        <div class="list-header">
          <div class="list-title">{{ t('app.fileHistory') }}</div>
          <div class="list-actions">
            <el-button
              v-if="!isBatchExportMode"
              class="batch-export-entry"
              :icon="Download"
              type="primary"
              link
              @click.stop="enterBatchExportMode"
            >
              {{ t('app.batchExport') }}
            </el-button>
            <span v-else class="batch-mode-label">{{ t('app.batchExportMode') }}</span>
            <el-button
              class="clear-history-btn"
              :icon="Delete"
              type="danger"
              link
              @click.stop="clearAllHistory"
            />
          </div>
        </div>
        <ul ref="fileListRef" class="file-list">
          <div class="highlight-slider" :style="sliderStyle"></div>
          <li
            v-for="file in historyFiles"
            :key="file.id"
            :data-file-id="file.id"
            class="file-item"
            :class="{
              active: file.id === selectedId,
              pinned: file.pinned,
              'batch-mode': isBatchExportMode,
              'batch-disabled': isBatchExportMode && file.status !== 'done',
            }"
            @click="handleFileItemClick(file)"
            @contextmenu.prevent.stop="handleFileItemContextMenu(file, $event)"
          >
            <el-checkbox
              v-if="isBatchExportMode"
              class="batch-checkbox"
              :model-value="selectedBatchFileIds.includes(file.id)"
              :disabled="file.status !== 'done'"
              @click.stop
              @change="handleBatchCheckboxChange(file, $event)"
            />
            <el-icon class="file-icon" :style="{ color: typeIconColor[file.type] }">
              <Tickets v-if="file.type === 'xlsx'" />
              <Files v-else-if="file.type === 'docx'" />
              <Document v-else />
            </el-icon>
            <span class="file-name">{{ file.name }}</span>
            <span class="status-tag" :class="file.status">
              <el-icon class="status-icon" :class="{ 'is-spin': file.status === 'processing' }">
                <CircleCheck v-if="file.status === 'done'" />
                <Loading v-else />
              </el-icon>
              {{ file.status === 'done' ? t('app.statusDone') : t('app.statusProcessing') }}
            </span>
            <el-button
              v-if="!isBatchExportMode"
              class="delete-item-btn"
              :icon="Delete"
              type="danger"
              link
              @click.stop="deleteFile(file.id)"
            />
          </li>
        </ul>

        <Transition name="batch-export">
          <div v-if="isBatchExportMode" class="batch-export-bar" @click.stop>
            <div class="batch-export-row">
              <span class="batch-count">{{ t('app.batchSelected', { count: selectedBatchCount }) }}</span>
              <el-select v-model="batchExportFormat" class="batch-format-select" size="small">
                <el-option label="Markdown" value="markdown" />
                <el-option label="HTML" value="html" />
                <el-option label="JSON" value="json" />
              </el-select>
            </div>
            <div class="batch-export-actions">
              <el-button type="primary" :icon="Download" @click="handleBatchExport">
                {{ t('app.batchExportBtn') }}
              </el-button>
              <el-button @click="exitBatchExportMode">
                {{ t('app.batchCancel') }}
              </el-button>
            </div>
          </div>
        </Transition>
      </div>

      <div class="sidebar-footer">
        <el-popover
          placement="top-start"
          :width="220"
          trigger="click"
          popper-class="geek-popover"
          :show-arrow="false"
          :offset="12"
        >
          <template #reference>
            <div class="sidebar-btn bottom-btn">
              <el-icon><Setting /></el-icon>
              <span>{{ t('app.settings') }}</span>
            </div>
          </template>

          <div class="geek-menu">
            <div class="menu-item" @click="openSetting('ai')">
              <el-icon><Cpu /></el-icon>
              <span>{{ t('app.tabAi') }}</span>
              <span class="shortcut">Ctrl+,</span>
            </div>
            <div class="menu-item" @click="openSetting('webhook')">
              <el-icon><Connection /></el-icon>
              <span>{{ t('app.tabWebhook') }}</span>
            </div>
            <div class="menu-item" @click="openSetting('parser')">
              <el-icon><Operation /></el-icon>
              <span>{{ t('app.tabParser') }}</span>
            </div>
            <div class="menu-item" @click="openSetting('prefs')">
              <el-icon><Setting /></el-icon>
              <span>{{ t('app.tabAppearance') }}</span>
            </div>
          </div>
        </el-popover>
      </div>
    </el-aside>

    <el-container class="main-wrap" @click="handleMainAreaClick">
      <el-header class="toolbar">
        <div class="toolbar-actions">
          <ClockThemeToggle class="custom-clock-toggle" />
          <el-button
            class="tool-btn"
            :icon="isEditMode ? View : EditPen"
            @click="isEditMode = !isEditMode"
          >
            {{ isEditMode ? t('app.modeRead') : t('app.modeEdit') }}
          </el-button>
          <el-dropdown
            class="export-dropdown"
            trigger="click"
            popper-class="export-dropdown-popper"
            @command="handleExport"
          >
            <el-button class="tool-btn export-btn" :icon="Download" plain>
              {{ $t('toolbar.export') }}
              <el-icon class="export-arrow"><ArrowDown /></el-icon>
            </el-button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item command="markdown">
                  <el-icon><Document /></el-icon>
                  <span>{{ t('app.exportMarkdown') }}</span>
                </el-dropdown-item>
                <el-dropdown-item command="html">
                  <el-icon><Files /></el-icon>
                  <span>{{ t('app.exportHtml') }}</span>
                </el-dropdown-item>
                <el-dropdown-item command="json">
                  <el-icon><Tickets /></el-icon>
                  <span>{{ t('app.exportJson') }}</span>
                </el-dropdown-item>
                <el-dropdown-item command="csv">
                  <el-icon><Grid /></el-icon>
                  <span>{{ t('app.exportCsv') }}</span>
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
          <el-button class="tool-btn" :icon="Grid" plain @click="openDataDialog">{{ $t('toolbar.extractTableData') }}</el-button>
          <el-button class="tool-btn ai-btn" :icon="MagicStick" @click.stop="openAssistant">{{ t('app.aiAnalysis') }}</el-button>
          <el-button
            v-if="updateState === 'available' && updateInfo"
            class="tool-btn update-badge"
            :icon="ArrowUp"
            @click="startUpdateDownload"
          >
            {{ t('app.updateAvailable') }} {{ updateInfo.version }}
          </el-button>
          <el-button
            v-if="updateState === 'downloading'"
            class="tool-btn update-progress"
            disabled
          >
            <span class="update-progress-bar" :style="{ width: updateProgress + '%' }" />
            <span class="update-progress-text">
              {{ updateProgress > 0 ? t('app.updateProgress', { percent: updateProgress }) : t('app.updateDownloading') + '...' }}
            </span>
          </el-button>
          <el-button
            v-if="updateState === 'installing'"
            class="tool-btn update-progress"
            disabled
          >
            {{ t('app.updateInstalling') }}...
          </el-button>
        </div>
      </el-header>

      <el-main class="preview">
        <div v-if="selectedId === null" class="empty-state">
          <el-empty :description="t('app.emptyHint')" />
        </div>

        <div v-else-if="currentSelectedFile?.status === 'processing'" class="converting-mask">
          <div class="parse-progress-panel">
            <div class="parse-progress-track" role="progressbar" :aria-valuenow="currentParseProgress.percent" aria-valuemin="0" aria-valuemax="100">
              <div class="parse-progress-fill" :style="currentParseProgressStyle"></div>
            </div>
            <div class="parse-progress-meta">
              <span class="parse-progress-stage">{{ getLocalizedParseStage(currentParseProgress.stage) }}...</span>
              <span class="parse-progress-percent">{{ currentParseProgress.percent }}%</span>
            </div>
          </div>
        </div>

        <template v-else-if="currentMarkdown !== null">
          <div class="workspace-container">
            <article
              v-if="!isEditMode"
              class="markdown-body rendered-markdown"
              v-html="renderedHtml"
            ></article>
            <Splitpanes v-else class="modern-splitpanes">
              <Pane size="50">
                <article
                  class="markdown-body rendered-markdown"
                  v-html="renderedHtml"
                ></article>
              </Pane>
              <Pane size="50">
                <div class="editor-container">
                  <VueMonacoEditor
                    :key="monacoEditorKey"
                    v-model:value="activeMarkdown"
                    :theme="isDark ? 'vs-dark' : 'vs'"
                    language="markdown"
                    :options="monacoEditorOptions"
                    class="monaco-wrapper"
                  />
                </div>
              </Pane>
            </Splitpanes>
          </div>
        </template>

        <div v-else class="empty-state">
          <el-empty :description="t('app.emptyHint')" />
        </div>
      </el-main>
    </el-container>

    <el-drawer
      v-model="drawerVisible"
      :title="t('app.aiAssistant')"
      direction="rtl"
      size="400px"
      :modal="false"
      :with-header="true"
      class="ai-drawer"
    >
      <div class="chat">
        <div class="chat-messages">
          <div
            v-for="msg in currentMessages"
            :key="msg.id"
            class="msg-row"
            :class="msg.role"
          >
            <div class="bubble">{{ msg.content }}</div>
          </div>
          <div v-if="thinking" class="msg-row ai">
            <div class="bubble thinking">
              <el-icon class="is-spin"><Loading /></el-icon>
              {{ t('app.aiThinking') }}...
            </div>
          </div>
        </div>

        <div class="chat-input">
          <el-input
            v-model="inputText"
            type="textarea"
            :rows="2"
            resize="none"
            :placeholder="`${t('app.aiInputPlaceholder')}…`"
            @keyup.enter.exact.prevent="sendMessage"
          />
          <el-button
            class="send-btn"
            type="primary"
            :icon="Promotion"
            circle
            :disabled="!inputText.trim() || thinking"
            @click="sendMessage"
          />
        </div>
      </div>
    </el-drawer>

    <el-dialog v-model="dialogVisible" :title="`${t('app.structuredExport')} (JSON/CSV)`" width="600px" class="data-dialog">
      <pre class="code-block"><code>{{ jsonOutput }}</code></pre>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('app.cancel') }}</el-button>
        <el-button type="primary" @click="copyData">{{ t('app.copyData') }}</el-button>
      </template>
    </el-dialog>

    <SettingsModal v-model:visible="showSettings" v-model:config="settings" :initial-tab="settingsInitialTab" />

    <div
      v-if="contextMenu.visible && contextMenuFile"
      class="file-context-menu"
      :style="{ left: `${contextMenu.left}px`, top: `${contextMenu.top}px` }"
      @click.stop
      @contextmenu.prevent
    >
      <button class="context-menu-item" type="button" @click="togglePinFile(contextMenuFile.id)">
        {{ contextMenuFile.pinned ? t('app.unpinFile') : t('app.pinFile') }}
      </button>
      <button class="context-menu-item" type="button" @click="renameFile(contextMenuFile.id)">{{ t('app.renameFile') }}</button>
      <button class="context-menu-item danger" type="button" @click="deleteFile(contextMenuFile.id)">{{ t('app.deleteFile') }}</button>
    </div>
  </el-container>
</template>

<style scoped>
* {
  box-sizing: border-box;
}

.workbench {
  height: 100vh;
  width: 100vw;
  overflow: hidden;
  background: var(--bg-main);
  color: var(--text-primary);
}

.sidebar {
  background: var(--bg-sidebar);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  padding: 20px 16px;
  gap: 20px;
}

.sidebar-upload {
  flex-shrink: 0;
}

.uploader :deep(.el-upload),
.uploader :deep(.el-upload-dragger) {
  width: 100%;
}

.uploader :deep(.el-upload-dragger) {
  border: 1.5px dashed var(--border-color);
  border-radius: 12px;
  background: var(--bg-card);
  padding: 20px 16px;
  transition: all 0.3s ease;
}

.uploader :deep(.el-upload-dragger:hover) {
  border-color: #409eff;
  background: linear-gradient(160deg, rgba(64, 158, 255, 0.05), rgba(64, 158, 255, 0.1));
  transform: translateY(-2px);
  box-shadow: 0 6px 18px rgba(64, 158, 255, 0.14);
}

.upload-inner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}

.upload-icon {
  font-size: 28px;
  color: #b4bccb;
  transition: color 0.3s ease;
}

.uploader :deep(.el-upload-dragger:hover) .upload-icon {
  color: #409eff;
}

.upload-text {
  font-size: 14px;
  color: var(--text-secondary);
  font-weight: 500;
}

.sidebar-list {
  flex: 1;
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-height: 0;
}

.sidebar-footer {
  flex-shrink: 0;
  padding-top: 2px;
  position: relative;
}

.sidebar-btn,
.sidebar-setting-trigger {
  width: 100%;
  height: 34px;
  display: flex;
  align-items: center;
  gap: 8px;
  border: none;
  border-radius: 10px;
  background: transparent;
  color: var(--text-primary);
  font-size: var(--editor-font-size);
  padding: 0 12px;
  cursor: pointer;
  transition:
    background 0.2s ease,
    color 0.2s ease,
    box-shadow 0.2s ease,
    transform 0.2s ease;
}

.sidebar-btn:hover,
.sidebar-setting-trigger:hover {
  background: var(--btn-hover);
  color: #1a6de0;
}

.bottom-btn {
  justify-content: flex-start;
}

.sidebar-settings-panel {
  position: absolute;
  left: 0;
  bottom: 42px;
  width: 100%;
  padding: 14px;
  border: 1px solid var(--border-color);
  border-radius: 8px;
  background: var(--bg-card);
  box-shadow: 0 12px 28px rgba(31, 45, 73, 0.12);
  z-index: 20;
  animation: settings-panel-in 0.16s ease;
}

.sidebar-settings-title {
  font-size: var(--editor-font-size);
  font-weight: 700;
  color: var(--text-primary);
  margin-bottom: 12px;
}

.sidebar-settings-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 12px;
  padding-top: 2px;
}

.setting-field.compact {
  display: grid;
  grid-template-columns: 78px minmax(0, 1fr);
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
}

.setting-field.compact .setting-label {
  font-size: 12px;
  line-height: 1.35;
  color: var(--text-secondary);
  white-space: nowrap;
  margin: 0;
}

.setting-field.compact :deep(.el-input),
.setting-field.compact :deep(.el-select) {
  width: 100%;
}

.setting-field.compact :deep(.el-input__wrapper),
.setting-field.compact :deep(.el-select__wrapper) {
  min-height: 32px;
  border-radius: 8px;
  box-shadow: 0 0 0 1px var(--border-color) inset;
  transition:
    box-shadow 0.18s ease,
    background 0.18s ease;
}

.setting-field.compact :deep(.el-input__wrapper:hover),
.setting-field.compact :deep(.el-select__wrapper:hover) {
  box-shadow: 0 0 0 1px var(--border-color) inset;
}

.setting-field.compact :deep(.el-input__wrapper.is-focus),
.setting-field.compact :deep(.el-select__wrapper.is-focused) {
  box-shadow: 0 0 0 1px #8b5cf6 inset, 0 0 0 3px rgba(139, 92, 246, 0.1);
}

.sidebar-settings-actions :deep(.el-button) {
  min-width: 64px;
  border-radius: 8px;
}

@keyframes settings-panel-in {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.list-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  min-height: 28px;
  padding: 0 2px 0 6px;
  flex-shrink: 0;
}

.list-title {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  letter-spacing: 1px;
  text-transform: uppercase;
}

.list-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
  width: 126px;
  min-width: 126px;
  height: 28px;
}

.batch-export-entry {
  width: 88px;
  height: 26px !important;
  padding: 0 4px !important;
  font-size: 12px !important;
  font-weight: 600 !important;
  justify-content: center;
}

.batch-mode-label {
  width: 88px;
  height: 26px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 600;
  color: #409eff;
  white-space: nowrap;
}

.clear-history-btn {
  opacity: 0.55;
  transition:
    opacity 0.2s ease,
    transform 0.2s ease;
}

.clear-history-btn:hover {
  opacity: 1;
  transform: translateY(-1px);
}

.file-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  position: relative;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
}

.highlight-slider {
  position: absolute;
  left: 0;
  width: 100%;
  background: rgba(64, 158, 255, 0.08);
  border-radius: 10px;
  z-index: 0;
  pointer-events: none;
  will-change: top, height;
}

.file-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 38px 10px 12px;
  border-radius: 10px;
  cursor: pointer;
  border: 1px solid transparent;
  position: relative;
  z-index: 1;
}

.file-item.batch-mode {
  cursor: pointer;
  padding-right: 12px;
}

.file-item.batch-disabled {
  opacity: 0.62;
  cursor: not-allowed;
}

.file-item:hover {
  background: var(--btn-hover);
}

.file-item.pinned {
  background: rgba(64, 158, 255, 0.05);
}

.file-item.active .file-name {
  color: #1a6de0;
  font-weight: 600;
  transition: color 0.3s ease;
}

.file-item:not(.active) .file-name {
  transition: color 0.3s ease;
}

.file-icon {
  font-size: 18px;
  flex-shrink: 0;
}

.batch-checkbox {
  flex-shrink: 0;
  height: 18px;
}

.file-name {
  flex: 1;
  font-size: 13px;
  color: var(--text-primary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.status-tag {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 20px;
  flex-shrink: 0;
}

.status-tag.done {
  color: #67c23a;
  background: rgba(103, 194, 58, 0.12);
}

.status-tag.processing {
  color: #e6a23c;
  background: rgba(230, 162, 60, 0.12);
}

.status-icon {
  font-size: 12px;
}

.delete-item-btn {
  position: absolute;
  top: 50%;
  right: 8px;
  opacity: 0;
  transform: translateY(-50%);
  transition: all 0.2s ease;
}

.file-item:hover .delete-item-btn {
  opacity: 1;
}

.batch-export-bar {
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 10px;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: var(--bg-card);
  box-shadow: 0 8px 20px rgba(31, 45, 73, 0.08);
}

.batch-export-enter-active,
.batch-export-leave-active {
  transition:
    transform 0.25s ease,
    opacity 0.25s ease;
}

.batch-export-enter-from,
.batch-export-leave-to {
  opacity: 0;
  transform: translateY(100%);
}

.batch-export-enter-to,
.batch-export-leave-from {
  opacity: 1;
  transform: translateY(0);
}

.batch-export-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.batch-count {
  min-width: 0;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  white-space: nowrap;
}

.batch-format-select {
  width: 118px;
  flex-shrink: 0;
}

.batch-export-actions {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto;
  gap: 8px;
}

.batch-export-actions :deep(.el-button) {
  min-width: 0;
  margin-left: 0;
}

.file-context-menu {
  position: fixed;
  z-index: 3000;
  min-width: 148px;
  padding: 6px;
  border: 1px solid var(--border-color);
  border-radius: 10px;
  background: var(--bg-card);
  box-shadow: 0 12px 32px rgba(31, 45, 73, 0.16);
  animation: context-menu-in 0.16s ease;
}

.context-menu-item {
  width: 100%;
  height: 34px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-primary);
  font-size: 13px;
  text-align: left;
  padding: 0 12px;
  cursor: pointer;
  transition:
    background 0.18s ease,
    color 0.18s ease,
    transform 0.18s ease;
}

.context-menu-item:hover {
  background: var(--btn-hover);
  color: #1a6de0;
  transform: translateX(2px);
}

.context-menu-item.danger {
  color: #f56c6c;
}

.context-menu-item.danger:hover {
  background: rgba(245, 108, 108, 0.1);
  color: #e34d59;
}

@keyframes context-menu-in {
  from {
    opacity: 0;
    transform: translateY(-4px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: translateY(0) scale(1);
  }
}

.main-wrap {
  background: var(--bg-card);
  flex-direction: column;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}

.toolbar {
  height: 72px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: flex-end;
  padding: 12px 56px 12px 32px;
  box-sizing: border-box;
  border-bottom: 1px solid var(--border-color);
  background: var(--bg-card);
}

.toolbar-actions {
  display: flex;
  gap: 10px;
  align-items: center;
  justify-content: flex-end;
  flex-wrap: nowrap;
}

.tool-btn {
  height: 36px !important;
  min-width: 92px;
  display: inline-flex !important;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 0 15px !important;
  border-radius: 8px !important;
  border: 1px solid var(--border-color) !important;
  background: var(--btn-bg) !important;
  color: var(--text-secondary) !important;
  font-size: 13px;
  font-weight: 500 !important;
  line-height: 1;
  transition: all 0.2s ease !important;
}

.toolbar-actions :deep(.el-button + .el-button) {
  margin-left: 0;
}

.custom-clock-toggle {
  transform: none;
  transform-origin: center right;
  margin-right: 16px;
  width: 80px;
  height: 36px;
  flex: 0 0 80px;
  position: relative;
  display: block;
  align-items: center;
  z-index: 999;
  overflow: visible;
  pointer-events: auto;
}

.tool-btn :deep(.el-icon) {
  flex-shrink: 0;
  font-size: 15px;
}

.export-dropdown {
  display: inline-flex;
  flex-shrink: 0;
}

.export-btn {
  min-width: 100px;
}

.export-arrow {
  margin-left: 2px;
  font-size: 12px;
  transition: transform 0.25s ease;
}

.export-dropdown :deep(.el-tooltip__trigger[aria-expanded='true']) .export-arrow {
  transform: rotate(180deg);
}

:global(.export-dropdown-popper) {
  animation: dropdown-fade 0.22s ease;
}

:global(.export-dropdown-popper .el-dropdown-menu) {
  padding: 6px;
  border-radius: 10px;
}

:global(.export-dropdown-popper .el-dropdown-menu__item) {
  display: flex;
  align-items: center;
  gap: 8px;
  border-radius: 8px;
  transition:
    background 0.2s ease,
    color 0.2s ease,
    transform 0.2s ease;
}

:global(.export-dropdown-popper .el-dropdown-menu__item .el-icon) {
  margin: 0;
  font-size: 15px;
  color: #64748b;
}

:global(.export-dropdown-popper .el-dropdown-menu__item:hover) {
  transform: translateX(2px);
}

@keyframes dropdown-fade {
  from {
    opacity: 0;
    transform: translateY(-6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.tool-btn:hover {
  background: var(--btn-hover) !important;
  border-color: var(--border-color) !important;
  color: var(--text-primary) !important;
}

.ai-btn {
  height: 36px !important;
  display: inline-flex !important;
  align-items: center !important;
  justify-content: center !important;
  background: var(--ai-btn-bg) !important;
  border: 1px solid var(--ai-btn-border) !important;
  color: var(--ai-btn-color) !important;
  font-weight: 600 !important;
  border-radius: 10px !important;
  min-width: 104px;
  padding: 0 16px !important;
  box-shadow: var(--ai-btn-shadow) !important;
  transition:
    transform 0.22s ease,
    box-shadow 0.22s ease,
    border-color 0.22s ease,
    filter 0.22s ease !important;
}

:global(html.dark) .ai-btn {
  background: var(--ai-btn-bg) !important;
  border-color: var(--ai-btn-border) !important;
  color: var(--ai-btn-color) !important;
  box-shadow: var(--ai-btn-shadow) !important;
}

.ai-btn :deep(.el-icon) {
  font-size: 16px;
  margin-right: 4px;
  color: var(--ai-icon-color);
  transition:
    color 0.22s ease,
    filter 0.22s ease;
}

:global(html.dark) .ai-btn :deep(.el-icon) {
  color: var(--ai-icon-color);
  filter: var(--ai-icon-filter);
}

.ai-btn:hover {
  background: var(--ai-btn-hover-bg) !important;
  border-color: var(--ai-btn-hover-border) !important;
  color: var(--ai-btn-hover-color) !important;
  transform: translateY(-2px);
  box-shadow: var(--ai-btn-hover-shadow) !important;
}

.update-badge {
  height: 36px !important;
  display: inline-flex !important;
  align-items: center !important;
  justify-content: center !important;
  background: #e8f0fe !important;
  border: 1px solid #90baf5 !important;
  color: #1a6de0 !important;
  font-weight: 600 !important;
  border-radius: 10px !important;
  min-width: 140px;
  padding: 0 16px !important;
  cursor: pointer !important;
  transition: all 0.25s ease !important;
  animation: update-badge-pulse 2s ease-in-out infinite;
}

.update-badge:hover {
  background: #d4e4fc !important;
  border-color: #5b9cf5 !important;
  transform: translateY(-1px);
}

.update-badge :deep(.el-icon) {
  font-size: 16px;
  margin-right: 6px;
  color: #1a6de0;
}

.update-progress {
  height: 36px !important;
  min-width: 140px;
  padding: 0 !important;
  border-radius: 10px !important;
  border: 1px solid #90baf5 !important;
  background: #e8f0fe !important;
  position: relative;
  overflow: hidden;
  cursor: default !important;
}

.update-progress-bar {
  position: absolute;
  left: 0;
  top: 0;
  height: 100%;
  background: linear-gradient(135deg, #409eff, #66b1ff);
  border-radius: 9px 0 0 9px;
  transition: width 0.3s ease;
  z-index: 0;
}

.update-progress-text {
  position: relative;
  z-index: 1;
  color: #3a4866;
  font-weight: 600;
  font-size: 13px;
}

:global(html.dark) .update-badge {
  background: #1a2d4a !important;
  border-color: #2a5da8 !important;
  color: #7bb8ff !important;
}

:global(html.dark) .update-badge :deep(.el-icon) {
  color: #7bb8ff;
}

:global(html.dark) .update-badge:hover {
  background: #1f3660 !important;
}

:global(html.dark) .update-progress {
  background: #1a2d4a !important;
  border-color: #2a5da8 !important;
}

:global(html.dark) .update-progress-text {
  color: #c8daf5;
}

@keyframes update-badge-pulse {
  0%, 100% { box-shadow: 0 0 0 0 rgba(64, 158, 255, 0.4); }
  50% { box-shadow: 0 0 0 6px rgba(64, 158, 255, 0); }
}

:global(html.dark) .ai-btn:hover {
  background: var(--ai-btn-hover-bg) !important;
  border-color: var(--ai-btn-hover-border) !important;
  color: var(--ai-btn-hover-color) !important;
  box-shadow: var(--ai-btn-hover-shadow) !important;
}

.ai-btn:active {
  transform: translateY(0);
  box-shadow:
    0 3px 10px rgba(139, 92, 246, 0.12),
    inset 0 1px 0 rgba(255, 255, 255, 0.5) !important;
}

.preview {
  padding: 24px;
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  background: var(--bg-main);
  display: flex;
  flex-direction: column;
}

.empty-state {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.converting-mask {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 24px;
}

.parse-progress-panel {
  width: min(420px, 100%);
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.parse-progress-track {
  width: 100%;
  height: 6px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(64, 158, 255, 0.12);
}

.parse-progress-fill {
  height: 100%;
  min-width: 8px;
  border-radius: inherit;
  background: linear-gradient(90deg, #409eff 0%, #67c23a 100%);
  box-shadow: 0 0 16px rgba(64, 158, 255, 0.24);
  transition: width 0.3s ease;
}

.parse-progress-meta {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 22px;
}

.parse-progress-stage {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 14px;
  color: var(--text-secondary);
}

.parse-progress-percent {
  flex-shrink: 0;
  min-width: 42px;
  text-align: right;
  font-size: 13px;
  font-weight: 700;
  color: #409eff;
}

.is-spin {
  animation: spin 1.2s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.markdown-body {
  width: 100%;
  max-width: none;
  margin: 0;
  background: var(--bg-card);
  padding: 32px 40px;
  border-radius: 12px;
  box-shadow: 0 4px 24px rgba(31, 45, 73, 0.04), 0 1px 3px rgba(31, 45, 73, 0.02);
  color: var(--text-primary);
  line-height: 1.75;
  min-height: 100%;
  box-sizing: border-box;
  animation: fade-in 0.4s ease;
  overflow-x: auto;
}

.workspace-container {
  flex: 1;
  height: 100%;
  min-width: 0;
  min-height: 0;
  width: 100%;
  background: var(--bg-main);
  overflow: hidden;
}

.workspace-container > .markdown-body {
  height: 100%;
  min-height: 0;
  overflow: auto;
}

.modern-splitpanes {
  height: 100%;
  width: 100%;
  min-height: 0;
  overflow: hidden;
}

.modern-splitpanes .editor-container,
.modern-splitpanes .markdown-body {
  height: 100%;
  min-height: 100%;
  margin: 0;
  overflow: auto;
  border-radius: 12px;
}

.modern-splitpanes .splitpanes__pane {
  padding: 0 8px;
  min-height: 0;
  overflow: hidden;
}

.modern-splitpanes .splitpanes__pane:first-child {
  padding-left: 0;
}

.modern-splitpanes .splitpanes__pane:last-child {
  padding-right: 0;
}

.modern-splitpanes :deep(.splitpanes__splitter) {
  background-color: transparent !important;
  width: 12px !important;
  cursor: col-resize !important;
  position: relative;
  transition: all 0.2s ease;
}

.modern-splitpanes :deep(.splitpanes__splitter:hover::before),
.modern-splitpanes :deep(.splitpanes__splitter.splitpanes__splitter__active::before) {
  content: '';
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  width: 4px;
  height: 40px;
  border-radius: 4px;
  background-color: #6366f1;
}

.editor-container {
  width: 100%;
  height: calc(100vh - 112px);
  background: var(--bg-card);
  border-radius: 12px;
  box-shadow: 0 4px 24px rgba(31, 45, 73, 0.04), 0 1px 3px rgba(31, 45, 73, 0.02);
  overflow: hidden;
  padding: 0;
}

.monaco-wrapper {
  width: 100%;
  height: 100%;
}

@keyframes fade-in {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.markdown-body h1 {
  font-size: 28px;
  font-weight: 700;
  margin: 0 0 16px;
  color: var(--text-primary);
  letter-spacing: 0.3px;
}

.markdown-body h2 {
  font-size: 19px;
  font-weight: 600;
  margin: 32px 0 14px;
  color: var(--text-primary);
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}

.markdown-body .summary {
  font-size: 15px;
  color: var(--text-secondary);
  background: var(--bg-main);
  border-left: 3px solid #409eff;
  padding: 14px 18px;
  border-radius: 0 8px 8px 0;
  margin: 0 0 8px;
}

.markdown-body table {
  width: 100%;
  min-width: 820px;
  border-collapse: collapse;
  font-size: 14px;
  table-layout: auto;
  border-radius: 10px;
  overflow: hidden;
  box-shadow: 0 1px 0 var(--border-color);
}

.markdown-body th {
  background: var(--bg-main);
  color: var(--text-secondary);
  font-weight: 600;
  text-align: left;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color);
}

.markdown-body td {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-color);
  color: var(--text-primary);
}

.markdown-body tbody tr {
  transition: background 0.2s ease;
}

.markdown-body tbody tr:hover {
  background: var(--btn-hover);
}

.markdown-body tbody tr:last-child td {
  border-bottom: none;
}

.rendered-markdown :deep(h1) {
  font-size: 28px;
  font-weight: 700;
  margin: 0 0 16px;
  color: var(--text-primary);
  letter-spacing: 0.3px;
}

.rendered-markdown :deep(h2) {
  font-size: 19px;
  font-weight: 600;
  margin: 32px 0 14px;
  color: var(--text-primary);
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}

.rendered-markdown :deep(h3) {
  font-size: 16px;
  font-weight: 600;
  margin: 24px 0 10px;
  color: var(--text-primary);
}

.rendered-markdown :deep(p) {
  margin: 0 0 12px;
  font-size: 14px;
  line-height: 1.75;
  color: var(--text-primary);
}

.rendered-markdown :deep(ul),
.rendered-markdown :deep(ol) {
  padding-left: 20px;
  margin: 0 0 14px;
}

.rendered-markdown :deep(li) {
  font-size: 14px;
  line-height: 1.75;
  color: var(--text-primary);
}

.rendered-markdown :deep(strong) {
  color: var(--text-primary);
  font-weight: 600;
}

.rendered-markdown :deep(table) {
  width: 100%;
  min-width: 900px;
  border-collapse: collapse;
  font-size: 14px;
  table-layout: auto;
  border-radius: 10px;
  overflow: hidden;
  box-shadow: 0 1px 4px rgba(31, 45, 73, 0.04);
  margin: 16px 0;
  border: 1px solid var(--border-color);
}

.rendered-markdown :deep(th) {
  background: var(--bg-main);
  color: var(--text-secondary);
  font-weight: 600;
  text-align: left;
  padding: 12px;
  border-bottom: 1px solid var(--border-color);
  border-right: none;
}

.rendered-markdown :deep(td) {
  padding: 12px;
  border-bottom: 1px solid var(--border-color);
  border-right: none;
  color: var(--text-primary);
}

.rendered-markdown :deep(th:last-child),
.rendered-markdown :deep(td:last-child) {
  border-right: none;
}

.rendered-markdown :deep(tbody tr:nth-child(even) td) {
  background: color-mix(in srgb, var(--bg-main) 70%, transparent);
}

.rendered-markdown :deep(tr:hover td) {
  background: var(--btn-hover);
}

.rendered-markdown :deep(tr:last-child td) {
  border-bottom: none;
}

.rendered-markdown :deep(code) {
  background: var(--btn-hover);
  color: #d63384;
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 13px;
  font-family: var(--editor-font-family);
}

.rendered-markdown :deep(pre) {
  background: #1e1e2e;
  color: #cdd6f4;
  border-radius: 10px;
  padding: 16px 20px;
  overflow-x: auto;
  font-size: 13px;
  line-height: 1.7;
  margin: 16px 0;
}

.rendered-markdown :deep(pre code) {
  background: none;
  color: inherit;
  padding: 0;
  font-size: inherit;
}

.rendered-markdown :deep(blockquote) {
  border-left: 3px solid #409eff;
  background: var(--bg-main);
  padding: 12px 16px;
  margin: 14px 0;
  border-radius: 0 8px 8px 0;
  color: var(--text-secondary);
}

.ai-drawer :deep(.el-drawer__header) {
  margin-bottom: 0;
  padding: 18px 20px;
  border-bottom: 1px solid var(--border-color);
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
  background: var(--bg-card);
}

.ai-drawer :deep(.el-drawer__body) {
  padding: 0;
  overflow: hidden;
  background: var(--bg-main);
}

.chat {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-main);
}

.chat-messages {
  flex: 1;
  overflow-y: auto;
  padding: 20px 18px;
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.msg-row {
  display: flex;
  animation: msg-in 0.35s cubic-bezier(0.22, 1, 0.36, 1);
}

.msg-row.ai {
  justify-content: flex-start;
}

.msg-row.user {
  justify-content: flex-end;
}

@keyframes msg-in {
  from {
    opacity: 0;
    transform: translateY(10px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.bubble {
  max-width: 78%;
  padding: 11px 15px;
  font-size: 14px;
  line-height: 1.6;
  border-radius: 14px;
  box-shadow: 0 2px 10px rgba(31, 45, 73, 0.06);
  word-break: break-word;
}

.msg-row.ai .bubble {
  background: var(--bg-card);
  color: var(--text-primary);
  border: 1px solid var(--border-color);
  border-top-left-radius: 4px;
}

.msg-row.user .bubble {
  background: linear-gradient(135deg, #409eff, #66b1ff);
  color: #ffffff;
  border-top-right-radius: 4px;
  box-shadow: 0 4px 14px rgba(64, 158, 255, 0.28);
}

.bubble.thinking {
  display: flex;
  align-items: center;
  gap: 7px;
  color: var(--text-secondary);
}

.chat-input {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 20px;
  border-top: 1px solid var(--border-color);
  background: var(--bg-card);
}

.chat-input :deep(.el-textarea__inner) {
  border-radius: 12px;
  box-shadow: none;
  border: 1px solid var(--border-color);
  background: var(--btn-bg);
  color: var(--text-primary);
  transition: all 0.25s ease;
  padding: 10px 14px;
  font-size: 14px;
  line-height: 1.5;
}

.chat-input :deep(.el-textarea__inner:focus) {
  border-color: #409eff;
  box-shadow: 0 0 0 3px rgba(64, 158, 255, 0.12);
}

.send-btn {
  width: 42px;
  height: 42px;
  flex-shrink: 0;
  font-size: 18px;
  box-shadow: 0 4px 12px rgba(64, 158, 255, 0.28);
  transition: all 0.25s ease;
}

.send-btn:hover:not(.is-disabled) {
  transform: translateY(-2px) scale(1.05);
  box-shadow: 0 6px 18px rgba(64, 158, 255, 0.38);
}

.data-dialog :deep(.el-dialog__header) {
  border-bottom: 1px solid var(--border-color);
  padding: 20px 24px;
  background: var(--bg-card);
  color: var(--text-primary);
}

.data-dialog :deep(.el-dialog__body) {
  padding: 24px;
  background: var(--bg-card);
  color: var(--text-primary);
}

.data-dialog :deep(.el-dialog__footer) {
  padding: 16px 24px;
  border-top: 1px solid var(--border-color);
  background: var(--bg-card);
}

.setting-field {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 18px;
}

.setting-field:last-child {
  margin-bottom: 0;
}

.setting-label {
  font-size: 14px;
  color: var(--text-primary);
  font-weight: 600;
}

.language-select {
  width: 100%;
}

.code-block {
  background: #1e1e2e;
  color: #cdd6f4;
  border-radius: 10px;
  padding: 20px 22px;
  overflow-x: auto;
  font-size: 13px;
  line-height: 1.8;
  margin: 0;
  white-space: pre;
  font-family: var(--editor-font-family);
}

.code-block code {
  font-family: inherit;
  background: none;
  padding: 0;
  color: inherit;
}
</style>

<style>
@font-face {
  font-family: 'Fira Code';
  src: url('./assets/fonts/fira-code-latin-400-normal.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'JetBrains Mono';
  src: url('./assets/fonts/jetbrains-mono-latin-400-normal.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Cascadia Code';
  src: url('./assets/fonts/cascadia-code-latin-400-normal.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

@font-face {
  font-family: 'Source Code Pro';
  src: url('./assets/fonts/source-code-pro-latin-400-normal.woff2') format('woff2');
  font-weight: 400;
  font-style: normal;
  font-display: swap;
}

/* 浅色模式变量 (默认) */
:root {
  --bg-main: #f8f9fb;
  --bg-card: #ffffff;
  --bg-sidebar: #ffffff;
  --text-primary: #2c3550;
  --text-secondary: #5a6478;
  --border-color: #e3e8f0;
  --btn-bg: #ffffff;
  --btn-hover: #f4f6f9;
  --editor-font-family: Consolas, monospace;
  --editor-font-size: 14px;
  --ai-btn-bg: linear-gradient(135deg, #ffffff, #f7f9fc);
  --ai-btn-hover-bg: #ffffff;
  --ai-btn-color: #5b3cc4;
  --ai-btn-hover-color: #5b3cc4;
  --ai-btn-border: rgba(99, 102, 241, 0.16);
  --ai-btn-hover-border: rgba(99, 102, 241, 0.34);
  --ai-btn-shadow: 0 1px 2px rgba(31, 45, 73, 0.04);
  --ai-btn-hover-shadow: 0 6px 16px rgba(99, 102, 241, 0.12), 0 1px 3px rgba(31, 45, 73, 0.04);
  --ai-icon-color: #6366f1;
  --ai-icon-filter: none;
  --el-bg-color: var(--bg-card);
  --el-bg-color-page: var(--bg-main);
  --el-bg-color-overlay: var(--bg-card);
  --el-text-color-primary: var(--text-primary);
  --el-text-color-regular: var(--text-secondary);
  --el-text-color-secondary: var(--text-secondary);
  --el-border-color: var(--border-color);
  --el-border-color-light: var(--border-color);
  --el-border-color-lighter: var(--border-color);
  --el-fill-color-blank: var(--btn-bg);
  --el-fill-color-light: var(--btn-hover);
}

/* 细滚动条，风格和 UI 一致 */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: transparent;
}

::-webkit-scrollbar-thumb {
  background: var(--border-color);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--text-secondary);
}

/* 暗黑模式变量 (由 useDark 触发) */
html.dark {
  --bg-main: #18181c;
  --bg-card: #222225;
  --bg-sidebar: #222225;
  --text-primary: #e3e3e8;
  --text-secondary: #9e9ea7;
  --border-color: #333336;
  --btn-bg: #2c2c2f;
  --btn-hover: #3a3a3e;
  --ai-btn-bg: linear-gradient(135deg, #2a2931, #25252a);
  --ai-btn-hover-bg: linear-gradient(135deg, #302d3a, #292832);
  --ai-btn-color: #c9c3df;
  --ai-btn-hover-color: #e4dcff;
  --ai-btn-border: rgba(167, 139, 250, 0.18);
  --ai-btn-hover-border: rgba(167, 139, 250, 0.34);
  --ai-btn-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.04), 0 1px 2px rgba(0, 0, 0, 0.18);
  --ai-btn-hover-shadow: 0 8px 18px rgba(0, 0, 0, 0.24), 0 0 18px rgba(139, 92, 246, 0.12);
  --ai-icon-color: #a78bfa;
  --ai-icon-filter: drop-shadow(0 0 5px rgba(167, 139, 250, 0.2));
  --el-bg-color: var(--bg-card);
  --el-bg-color-page: var(--bg-main);
  --el-bg-color-overlay: var(--bg-card);
  --el-text-color-primary: var(--text-primary);
  --el-text-color-regular: var(--text-secondary);
  --el-text-color-secondary: var(--text-secondary);
  --el-border-color: var(--border-color);
  --el-border-color-light: var(--border-color);
  --el-border-color-lighter: var(--border-color);
  --el-fill-color-blank: var(--btn-bg);
  --el-fill-color-light: var(--btn-hover);
}

html,
body,
#app {
  height: 100%;
  margin: 0;
  overflow: hidden;
  background: var(--bg-main);
  color: var(--text-primary);
}

/* View Transition 核心扩散动画层级控制 */
::view-transition-old(root),
::view-transition-new(root) {
  animation: none;
  mix-blend-mode: normal;
  pointer-events: none;
}

/* 始终让新主题快照位于上层，从点击处向外扩散，避免白黑切换闪屏 */
::view-transition-old(root) {
  z-index: 1;
}

::view-transition-new(root) {
  z-index: 2147483646;
}

.el-dropdown-menu,
.el-popper {
  background: var(--bg-card) !important;
  border-color: var(--border-color) !important;
}

.el-dropdown-menu__item {
  color: var(--text-primary) !important;
}

.el-dropdown-menu__item:not(.is-disabled):focus,
.el-dropdown-menu__item:not(.is-disabled):hover {
  background: var(--btn-hover) !important;
  color: var(--text-primary) !important;
}

.el-input__wrapper,
.el-select__wrapper {
  background: var(--btn-bg) !important;
  box-shadow: 0 0 0 1px var(--border-color) inset !important;
}

.el-input__inner,
.el-textarea__inner,
.el-select__placeholder {
  color: var(--text-primary) !important;
}

.el-dialog,
.el-drawer {
  background: var(--bg-card) !important;
  color: var(--text-primary) !important;
}

.toolbar .ai-btn {
  background: var(--ai-btn-bg) !important;
  border-color: var(--ai-btn-border) !important;
  color: var(--ai-btn-color) !important;
  box-shadow: var(--ai-btn-shadow) !important;
}

.toolbar .ai-btn .el-icon {
  color: var(--ai-icon-color) !important;
  filter: var(--ai-icon-filter);
}

.toolbar .ai-btn:hover {
  background: var(--ai-btn-hover-bg) !important;
  border-color: var(--ai-btn-hover-border) !important;
  color: var(--ai-btn-hover-color) !important;
  box-shadow: var(--ai-btn-hover-shadow) !important;
}

/* 彻底重写 el-popover 的默认样式，打造悬浮磨砂质感 */
.el-popover.geek-popover {
  padding: 6px !important;
  border-radius: 12px !important;
  background-color: var(--bg-card) !important;
  border: 1px solid var(--border-color) !important;
  box-shadow: 0 12px 32px rgba(0, 0, 0, 0.1), 0 2px 6px rgba(0, 0, 0, 0.05) !important;
}

.geek-menu {
  display: flex;
  flex-direction: column;
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 8px;
  cursor: pointer;
  color: var(--text-primary);
  font-size: 13px;
  transition: background-color 0.15s ease;
}

.menu-item:hover {
  background-color: var(--btn-hover);
}

.menu-item .el-icon {
  font-size: 16px;
  color: var(--text-secondary);
}

.menu-item .shortcut {
  margin-left: auto;
  font-size: 12px;
  color: var(--text-secondary);
  opacity: 0.7;
}

.menu-divider {
  height: 1px;
  background-color: var(--border-color);
  margin: 6px 0;
}

.menu-item.danger:hover {
  background-color: #fee2e2;
  color: #dc2626;
}

html.dark .menu-item.danger:hover {
  background-color: rgba(220, 38, 38, 0.2);
  color: #f87171;
}

/* 现代风格确认弹窗 */
.modern-confirm-dialog {
  border-radius: 16px !important;
  padding-bottom: 24px !important;
  border: 1px solid var(--border-color) !important;
  background: var(--bg-card) !important;
  box-shadow: 0 10px 30px rgba(31, 45, 73, 0.1) !important;
}

.modern-confirm-dialog .el-message-box__header {
  padding-top: 24px;
}

.modern-confirm-dialog .el-message-box__title {
  font-weight: 600;
  color: var(--text-primary);
}

.modern-confirm-dialog .el-message-box__content {
  color: var(--text-secondary);
  font-size: 14px;
  margin-top: 8px;
}

.modern-confirm-dialog .el-message-box__btns {
  padding-top: 20px;
  gap: 12px;
}

/* 取消按钮柔和化 */
.modern-cancel-btn {
  border-radius: 8px !important;
  border: 1px solid var(--border-color) !important;
  color: var(--text-secondary) !important;
  background: var(--btn-bg) !important;
  font-weight: 500 !important;
  transition: all 0.2s ease !important;
}

.modern-cancel-btn:hover {
  background: var(--btn-hover) !important;
  color: var(--text-primary) !important;
}

/* 危险按钮高级感（降低饱和度） */
.modern-danger-btn {
  border-radius: 8px !important;
  background: #ff4d4f !important;
  border: none !important;
  font-weight: 500 !important;
  box-shadow: 0 4px 12px rgba(255, 77, 79, 0.2) !important;
  transition: all 0.2s ease !important;
}

.modern-danger-btn:hover {
  background: #ff7875 !important;
  box-shadow: 0 6px 16px rgba(255, 77, 79, 0.3) !important;
  transform: translateY(-1px);
}
</style>
