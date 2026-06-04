<template>
  <el-drawer
    v-model="visible"
    :title="t('settingsModal.settingsTitle')"
    direction="ltr"
    :size="680"
    :with-header="false"
    class="settings-drawer"
    destroy-on-close
    @open="loadSettings"
    @opened="updateNavSlider(activeTab)"
  >
    <div class="mac-settings-container">
      <div class="mac-sidebar">
        <div class="sidebar-header">
          <h2>{{ t('settingsModal.settingsTitle') }}</h2>
        </div>
        <div ref="navListRef" class="nav-list">
          <div class="nav-slider" :style="navSliderStyle"></div>
          <div
            v-for="tab in tabs"
            :key="tab.id"
            :data-tab-id="tab.id"
            class="nav-item"
            :class="{ active: activeTab === tab.id }"
            @click="setActiveTab(tab.id)"
          >
            <el-icon class="nav-icon"><component :is="tab.icon" /></el-icon>
            <span>{{ tab.name }}</span>
          </div>
        </div>
      </div>

      <div class="mac-content">
        <div class="content-header">
          <h3>{{ tabs.find((tab) => tab.id === activeTab)?.name }}</h3>
          <p class="subtitle">{{ t('settingsModal.settingsSubtitle') }}</p>
        </div>

        <div class="content-body">
          <div v-if="activeTab === 'ai'" class="setting-section fade-in">
            <div class="form-group">
              <label>{{ t('settingsModal.provider') }}</label>
              <el-select
                v-model="settings.provider"
                class="mac-select"
                :placeholder="t('settingsModal.providerPlaceholder')"
                @change="handleProviderChange"
              >
                <el-option
                  v-for="provider in providerOptions"
                  :key="provider.value"
                  :label="provider.label"
                  :value="provider.value"
                />
              </el-select>
            </div>
            <div class="form-group">
              <label>API Key</label>
              <el-input
                v-model="settings.apiKeys[settings.provider]"
                :placeholder="`${t('settingsModal.apiKeyPlaceholder')}...`"
                type="password"
                show-password
              />
            </div>
            <div class="form-group">
              <label>Base URL</label>
              <el-input
                v-model="settings.baseUrl"
                :disabled="settings.provider !== 'custom'"
                :placeholder="`${t('settingsModal.baseUrlPlaceholder')}...`"
              />
            </div>
            <div class="form-group">
              <label>{{ t('settingsModal.model') }}</label>
              <div class="model-row">
                <el-select
                  v-model="settings.selectedModel"
                  class="model-select"
                  :placeholder="t('settingsModal.modelPlaceholder')"
                >
                  <el-option
                    v-for="model in modelOptions"
                    :key="model"
                    :label="model"
                    :value="model"
                  />
                  <el-option :label="`${t('settingsModal.modelCustom')}...`" value="custom" />
                </el-select>
                <el-button
                  class="mac-btn"
                  :loading="detectingModels"
                  @click="detectAvailableModels"
                >
                  {{ t('settingsModal.detectModels') }}
                </el-button>
              </div>
            </div>
            <div v-if="settings.selectedModel === 'custom'" class="form-group">
              <label>{{ t('settingsModal.customModel') }}</label>
              <el-input v-model="settings.customModel" :placeholder="t('settingsModal.customModelPlaceholder')" />
            </div>
          </div>

          <div v-if="activeTab === 'webhook'" class="setting-section fade-in">
            <div class="form-group">
              <label>{{ t('settingsModal.webhookUrl') }}</label>
              <el-input v-model="settings.webhookUrl" :placeholder="t('settingsModal.webhookPlaceholder')" />
            </div>
            <div class="form-group switch-group">
              <span>{{ t('settingsModal.autoPushLabel') }}</span>
              <el-switch v-model="settings.autoPush" />
            </div>
          </div>

          <div v-if="activeTab === 'parser'" class="setting-section fade-in">
            <div class="form-group switch-group">
              <span>{{ t('settingsModal.strictAlign') }}</span>
              <el-switch v-model="settings.strictAlign" />
            </div>
            <div class="form-group switch-group">
              <span>{{ t('settingsModal.enableOcr') }}</span>
              <el-switch v-model="settings.enableOcr" />
            </div>
            <div class="form-group switch-group">
              <span>{{ t('settingsModal.specTolerance') }}</span>
              <el-switch v-model="settings.specTolerance" />
            </div>
            <div class="form-group switch-group">
              <span>{{ t('settingsModal.flattenHeaders') }}</span>
              <el-switch v-model="settings.flattenHeaders" />
            </div>
          </div>

          <div v-if="activeTab === 'prefs'" class="setting-section fade-in">
            <div class="form-group">
              <label>{{ t('settingsModal.language') }}</label>
              <el-select v-model="settings.language" class="language-select">
                <el-option :label="t('settingsModal.languageZh')" value="zh-CN" />
                <el-option label="English" value="en-US" />
              </el-select>
            </div>
            <div class="form-group">
              <label>{{ t('settingsModal.fontSize') }}</label>
              <el-input-number v-model="settings.fontSize" :min="12" :max="24" controls-position="right" />
            </div>
            <div class="form-group">
              <label>{{ t('settingsModal.fontFamily') }}</label>
              <el-select v-model="settings.fontFamily" :placeholder="t('settingsModal.fontFamily')" class="mac-select">
                <el-option
                  v-for="font in fontOptions"
                  :key="font.value"
                  :label="font.label"
                  :value="font.value"
                >
                  <span :style="{ fontFamily: font.value }">{{ font.label }}</span>
                </el-option>
              </el-select>
            </div>
            <div class="form-group switch-group">
              <span>{{ t('settingsModal.autoSave') }}</span>
              <el-switch v-model="settings.autoSave" />
            </div>
          </div>
        </div>

        <div class="content-footer">
          <el-button class="mac-btn" @click="visible = false">{{ t('settingsModal.cancel') }}</el-button>
          <el-button class="mac-btn primary" type="primary" @click="saveSettings">{{ t('settingsModal.saveSettings') }}</el-button>
        </div>
      </div>
    </div>
  </el-drawer>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { Connection, Cpu, Operation, Setting } from '@element-plus/icons-vue'

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

const props = defineProps<{
  initialTab?: string
}>()

const visible = defineModel<boolean>('visible', { default: false })
const config = defineModel<AppSettingsState>('config', { required: true })
const { locale, t } = useI18n({ useScope: 'global' })

const activeTab = ref(props.initialTab ?? 'ai')
const navListRef = ref<HTMLElement | null>(null)
const navSliderStyle = ref<Record<string, string>>({
  top: '0px',
  height: '0px',
  opacity: '0',
})

const providerOptions = computed(() => [
  { label: 'DeepSeek', value: 'deepseek', baseUrl: 'https://api.deepseek.com' },
  { label: 'OpenAI', value: 'openai', baseUrl: 'https://api.openai.com' },
  { label: 'Gemini', value: 'gemini', baseUrl: 'https://generativelanguage.googleapis.com/v1beta' },
  { label: t('settingsModal.providerSiliconFlow'), value: 'siliconflow', baseUrl: 'https://api.siliconflow.cn' },
  { label: t('settingsModal.providerCustom'), value: 'custom', baseUrl: '' },
])

const modelOptions = ref<string[]>([])
const detectingModels = ref(false)
const defaultApiKeys: Record<string, string> = {
  deepseek: '',
  openai: '',
  gemini: '',
  siliconflow: '',
  custom: '',
}

const createSettingsDraft = (source: Partial<AppSettingsState> = {}): AppSettingsState => ({
  provider: source.provider || 'deepseek',
  apiKeys: {
    ...defaultApiKeys,
    ...(source.apiKeys ?? {}),
  },
  baseUrl: source.baseUrl || '',
  selectedModel: source.selectedModel || '',
  customModel: source.customModel || '',
  webhookUrl: source.webhookUrl || '',
  autoPush: source.autoPush ?? false,
  strictAlign: source.strictAlign ?? true,
  enableOcr: source.enableOcr ?? false,
  specTolerance: source.specTolerance ?? false,
  flattenHeaders: source.flattenHeaders ?? false,
  language: source.language || 'zh-CN',
  fontSize: source.fontSize ?? 14,
  fontFamily: source.fontFamily || 'Consolas, monospace',
  autoSave: source.autoSave ?? true,
})

const settings = ref<AppSettingsState>(createSettingsDraft(config.value))
const loadingDraft = ref(false)

const tabs = computed(() => [
  { id: 'ai', name: t('settingsModal.tabAi'), icon: Cpu },
  { id: 'webhook', name: t('settingsModal.tabWebhook'), icon: Connection },
  { id: 'parser', name: t('settingsModal.tabParser'), icon: Operation },
  { id: 'prefs', name: t('settingsModal.tabAppearance'), icon: Setting },
])

const fontOptions = computed(() => [
  { label: t('settingsModal.fontConsolas'), value: 'Consolas, monospace' },
  { label: t('settingsModal.fontFira'), value: '"Fira Code", monospace' },
  { label: t('settingsModal.fontJetbrains'), value: '"JetBrains Mono", monospace' },
  { label: t('settingsModal.fontCascadia'), value: '"Cascadia Code", monospace' },
  { label: t('settingsModal.fontSource'), value: '"Source Code Pro", monospace' },
  { label: t('settingsModal.fontMenlo'), value: 'Menlo, Monaco, monospace' },
])

watch(
  () => props.initialTab,
  (tab) => {
    if (tab) setActiveTab(tab)
  },
)

watch(
  () => settings.value.provider,
  () => {
    if (loadingDraft.value) return
    modelOptions.value = []
    settings.value.selectedModel = ''
    settings.value.customModel = ''
  },
)

const updateNavSlider = async (tabId: string) => {
  await nextTick()
  const listEl = navListRef.value
  if (!listEl) return
  const item = listEl.querySelector(`[data-tab-id="${tabId}"]`) as HTMLElement | null
  if (!item) return

  const listRect = listEl.getBoundingClientRect()
  const itemRect = item.getBoundingClientRect()
  const newTop = itemRect.top - listRect.top
  const newHeight = itemRect.height
  const oldTop = parseFloat(navSliderStyle.value.top || '0')
  const oldHeight = parseFloat(navSliderStyle.value.height || '0')

  if (oldHeight === 0 || navSliderStyle.value.opacity === '0') {
    navSliderStyle.value = {
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

  navSliderStyle.value = {
    top: `${stretchTop}px`,
    height: `${stretchHeight}px`,
    opacity: '1',
    transition: 'top 0.26s cubic-bezier(0.4, 0, 0.2, 1), height 0.26s cubic-bezier(0.4, 0, 0.2, 1)',
  }

  window.setTimeout(() => {
    navSliderStyle.value = {
      top: `${newTop}px`,
      height: `${newHeight}px`,
      opacity: '1',
      transition: 'top 0.4s cubic-bezier(0.34, 1.56, 0.64, 1), height 0.4s cubic-bezier(0.34, 1.56, 0.64, 1)',
    }
  }, 240)
}

const setActiveTab = (tabId: string) => {
  activeTab.value = tabId
  updateNavSlider(tabId)
}

const loadSettings = () => {
  loadingDraft.value = true
  settings.value = createSettingsDraft(config.value)
  modelOptions.value = settings.value.selectedModel && settings.value.selectedModel !== 'custom'
    ? [settings.value.selectedModel]
    : []

  const nextTab = props.initialTab ?? activeTab.value
  activeTab.value = nextTab
  updateNavSlider(nextTab)
  if (!settings.value.provider) {
    settings.value.provider = 'deepseek'
  }
  settings.value.apiKeys = {
    ...defaultApiKeys,
    ...(settings.value.apiKeys ?? {}),
  }
  const provider = providerOptions.value.find((item) => item.value === settings.value.provider)
  if (!settings.value.baseUrl && provider?.baseUrl) {
    settings.value.baseUrl = provider.baseUrl
  }

  nextTick(() => {
    loadingDraft.value = false
  })
}

const handleProviderChange = (provider: string) => {
  settings.value.apiKeys = {
    ...defaultApiKeys,
    ...(settings.value.apiKeys ?? {}),
  }
  modelOptions.value = []
  settings.value.selectedModel = ''
  settings.value.customModel = ''
  const nextProvider = providerOptions.value.find((item) => item.value === provider)
  if (nextProvider?.baseUrl) {
    settings.value.baseUrl = nextProvider.baseUrl
  }
}

const detectAvailableModels = async () => {
  if (settings.value.provider === 'gemini') {
    ElMessage.info(`${t('settingsModal.geminiManualHint')}，例如 gemini-2.0-flash`)
    return
  }

  const apiKey = (settings.value.apiKeys?.[settings.value.provider] ?? '').trim()
  const baseUrl = settings.value.baseUrl.trim()

  if (!apiKey) {
    ElMessage.warning(t('settingsModal.apiKeyRequired'))
    return
  }

  if (!baseUrl) {
    ElMessage.warning(t('settingsModal.baseUrlRequired'))
    return
  }

  const controller = new AbortController()
  const timeoutId = window.setTimeout(() => controller.abort(), 10000)
  detectingModels.value = true

  try {
    const response = await fetch(`${baseUrl.replace(/\/+$/, '')}/models`, {
      method: 'GET',
      headers: {
        Authorization: `Bearer ${apiKey}`,
      },
      signal: controller.signal,
    })

    if (!response.ok) {
      const errorText = await response.text()
      throw new Error(`${t('settingsModal.detectFailed')}：${response.status} ${response.statusText}${errorText ? ` - ${errorText}` : ''}`)
    }

    const payload = await response.json() as {
      data?: Array<{
        id?: unknown
      }>
    }
    const models = (payload.data ?? [])
      .map((item) => item.id)
      .filter((id): id is string => typeof id === 'string' && id.trim() !== '')

    if (models.length === 0) {
      throw new Error(t('settingsModal.noModelsFound'))
    }

    modelOptions.value = models
    settings.value.selectedModel = models[0]
    ElMessage.success(t('settingsModal.modelsDetected', { count: models.length }))
  } catch (error) {
    const message = error instanceof Error && error.name === 'AbortError'
      ? t('settingsModal.detectTimeout')
      : error instanceof Error
        ? error.message
        : String(error)
    ElMessage.error(message)
  } finally {
    window.clearTimeout(timeoutId)
    detectingModels.value = false
  }
}

const saveSettings = () => {
  const nextSettings = createSettingsDraft(settings.value)
  config.value = nextSettings
  locale.value = nextSettings.language
  ElMessage.success(t('settingsModal.settingsSaved'))
  visible.value = false
}
</script>

<style scoped>
.mac-settings-container {
  display: flex !important;
  flex-direction: row !important;
  height: 100%;
  width: 100%;
  background-color: var(--bg-card);
}

.mac-sidebar {
  width: 240px !important;
  flex-shrink: 0 !important;
  background-color: var(--bg-main);
  border-right: 1px solid var(--border-color);
  display: flex;
  flex-direction: column;
  height: 100%;
}

.sidebar-header {
  padding: 24px 20px 16px;
}

.sidebar-header h2 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
  color: var(--text-primary);
}

.nav-list {
  display: flex !important;
  flex-direction: column !important;
  gap: 8px;
  padding: 0 16px;
  position: relative;
}

.nav-slider {
  position: absolute;
  left: 16px;
  right: 16px;
  background: rgba(64, 158, 255, 0.08);
  border-radius: 10px;
  z-index: 0;
  pointer-events: none;
  will-change: top, height;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 12px;
  border-radius: 10px;
  font-size: 14px;
  color: var(--text-secondary);
  cursor: pointer;
  position: relative;
  z-index: 1;
  transition:
    background 0.2s ease,
    color 0.3s ease;
}

.nav-item:hover {
  background-color: var(--btn-hover);
}

.nav-item.active {
  color: #1a6de0;
  font-weight: 600;
}

.nav-item.active:hover {
  background: transparent;
}

.mac-content {
  flex: 1 !important;
  min-width: 0;
  display: flex;
  flex-direction: column;
  position: relative;
  height: 100%;
  background-color: var(--bg-card);
}

.content-header {
  padding: 32px 40px 24px;
  border-bottom: 1px solid var(--border-color);
}

.content-header h3 {
  margin: 0 0 6px;
  font-size: 20px;
  font-weight: 600;
  color: var(--text-primary);
}

.content-header .subtitle {
  margin: 0;
  font-size: 13px;
  color: var(--text-secondary);
}

.content-body {
  flex: 1;
  padding: 32px 40px;
  overflow-y: auto;
}

.form-group {
  margin-bottom: 28px;
}

.form-group label {
  display: block;
  margin-bottom: 10px;
  font-size: 13px;
  font-weight: 500;
  color: var(--text-primary);
}

.switch-group {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  background-color: var(--bg-main);
  border-radius: 10px;
  border: 1px solid var(--border-color);
  margin-bottom: 16px;
}

.switch-group span {
  font-size: 14px;
  font-weight: 500;
  color: var(--text-primary);
}

.content-footer {
  padding: 20px 40px;
  border-top: 1px solid var(--border-color);
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  background-color: var(--bg-main);
}

.mac-btn {
  border-radius: 8px !important;
  padding: 8px 20px !important;
  font-weight: 500 !important;
}

.language-select {
  width: 100%;
}

.mac-select {
  width: 100%;
}

.model-row {
  display: flex;
  gap: 12px;
  align-items: center;
}

.model-select {
  flex: 1;
  min-width: 0;
}

.fade-in {
  animation: fadeIn 0.3s ease-out;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(5px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

:deep(.el-drawer__body) {
  padding: 0 !important;
  overflow: hidden !important;
  display: flex;
  flex-direction: column;
}

:deep(.el-drawer) {
  background-color: var(--bg-card);
  height: 100%;
}

:deep(.el-input__wrapper),
:deep(.el-select__wrapper) {
  background-color: var(--bg-main);
  box-shadow: 0 0 0 1px var(--border-color) inset;
}

:deep(.el-input__inner),
:deep(.el-select__placeholder) {
  color: var(--text-primary);
}
</style>
