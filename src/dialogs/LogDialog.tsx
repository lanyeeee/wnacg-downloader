import { computed, defineComponent, onMounted, ref, watch } from 'vue'
import {
  darkTheme,
  useNotification,
  NModal,
  NDialog,
  NInputGroup,
  NInput,
  NSelect,
  NButton,
  NCheckbox,
  NConfigProvider,
  NVirtualList,
} from 'naive-ui'
import { commands, events, LogEvent, LogLevel } from '../bindings.ts'
import { appDataDir } from '@tauri-apps/api/path'
import { path } from '@tauri-apps/api'
import { useStore } from '../store.ts'

type LogRecord = LogEvent & { id: number; formatedLog: string }

export default defineComponent({
  name: 'LogDialog',
  props: {
    showing: {
      type: Boolean,
      required: true,
    },
  },
  emits: {
    'update:showing': (_value: boolean) => true,
  },
  setup(props, { emit }) {
    const store = useStore()

    const notification = useNotification()

    let nextLogRecordId = 1

    const logRecords = ref<LogRecord[]>([])
    const searchText = ref<string>('')
    const selectedLevel = ref<LogLevel>('INFO')
    const logsDirSize = ref<number>(0)

    const formatedLogsDirSize = computed<string>(() => {
      const units = ['B', 'KB', 'MB']
      let size = logsDirSize.value
      let unitIndex = 0

      while (size >= 1024 && unitIndex < 2) {
        size /= 1024
        unitIndex++
      }

      // 保留两位小数
      return `${size.toFixed(2)} ${units[unitIndex]}`
    })

    const filteredLogs = computed<LogRecord[]>(() => {
      return logRecords.value.filter(({ level, formatedLog }) => {
        // 定义日志等级的优先级顺序
        const logLevelPriority = {
          TRACE: 0,
          DEBUG: 1,
          INFO: 2,
          WARN: 3,
          ERROR: 4,
        }
        // 首先按日志等级筛选
        if (logLevelPriority[level] < logLevelPriority[selectedLevel.value]) {
          return false
        }
        // 然后按搜索文本筛选
        if (searchText.value === '') {
          return true
        }

        return formatedLog.toLowerCase().includes(searchText.value.toLowerCase())
      })
    })

    watch(
      () => props.showing,
      async (showing) => {
        if (showing) {
          const result = await commands.getLogsDirSize()
          if (result.status === 'error') {
            console.error(result.error)
            return
          }
          logsDirSize.value = result.data
        }
      },
    )

    onMounted(async () => {
      await events.logEvent.listen(async ({ payload: logEvent }) => {
        logRecords.value.push({
          ...logEvent,
          id: nextLogRecordId++,
          formatedLog: formatLogEvent(logEvent),
        })
        const { level, fields } = logEvent
        if (level === 'ERROR') {
          notification.error({
            title: fields['err_title'] as string,
            description: fields['message'] as string,
            duration: 0,
          })
        }
      })
    })

    function formatLogEvent(logEvent: LogEvent): string {
      const { timestamp, level, fields, target, filename, line_number } = logEvent
      const fields_str = Object.entries(fields)
        .sort(([key1], [key2]) => key1.localeCompare(key2))
        .map(([key, value]) => `${key}=${value}`)
        .join(' ')
      return `${timestamp} ${level} ${target}: ${filename}:${line_number} ${fields_str}`
    }

    function getLevelStyles(level: LogLevel) {
      switch (level) {
        case 'TRACE':
          return 'text-gray-400'
        case 'DEBUG':
          return 'text-green-400'
        case 'INFO':
          return 'text-blue-400'
        case 'WARN':
          return 'text-yellow-400'
        case 'ERROR':
          return 'text-red-400'
      }
    }

    const logLevelOptions = [
      { value: 'TRACE', label: 'TRACE' },
      { value: 'DEBUG', label: 'DEBUG' },
      { value: 'INFO', label: 'INFO' },
      { value: 'WARN', label: 'WARN' },
      { value: 'ERROR', label: 'ERROR' },
    ]

    function clearLogRecords() {
      logRecords.value = []
      nextLogRecordId = 1
    }

    async function showLogsDirInFileManager() {
      const logsDir = await path.join(await appDataDir(), '日志')
      const result = await commands.showPathInFileManager(logsDir)
      if (result.status === 'error') {
        console.error(result.error)
      }
    }

    return () =>
      store.config !== undefined && (
        <NModal show={props.showing} onUpdate:show={(value) => emit('update:showing', value)}>
          <NDialog
            showIcon={false}
            title={`日志目录总大小：${formatedLogsDirSize.value}`}
            onClose={() => emit('update:showing', false)}
            style="width: 95%">
            <div class="mb-2 flex flex-wrap gap-2">
              <NInputGroup class="w-100">
                <NInput
                  size="small"
                  value={searchText.value}
                  onUpdate:value={(value) => (searchText.value = value)}
                  placeholder="搜索日志..."
                  clearable
                />
                <NSelect
                  size="small"
                  value={selectedLevel.value}
                  onUpdate:value={(value) => (selectedLevel.value = value as LogLevel)}
                  options={logLevelOptions}
                  style="width: 120px"
                />
              </NInputGroup>

              <div class="flex flex-wrap gap-2 ml-auto items-center">
                <NButton size="small" onClick={showLogsDirInFileManager}>
                  打开日志目录
                </NButton>
                <NCheckbox
                  checked={store.config.enableFileLogger}
                  onUpdate:checked={(value) => {
                    if (store.config) {
                      store.config.enableFileLogger = value
                    }
                  }}>
                  输出文件日志
                </NCheckbox>
              </div>
            </div>

            <NConfigProvider theme={darkTheme} theme-overrides={{ Scrollbar: { width: '8px' } }}>
              <NVirtualList
                class="h-[calc(100vh-300px)] overflow-hidden bg-gray-900"
                itemSize={42}
                itemResizable
                items={filteredLogs.value}
                scrollbar-props={{ trigger: 'none' }}>
                {{
                  default: ({ item: { level, formatedLog } }: { item: LogRecord }) => {
                    return (
                      <div class={['py-1 px-3 hover:bg-white/10 whitespace-pre-wrap mr-4', getLevelStyles(level)]}>
                        {formatedLog}
                      </div>
                    )
                  },
                }}
              </NVirtualList>
            </NConfigProvider>

            <div class="pt-1 flex">
              <NButton ghost class="ml-auto" size="small" type="error" onClick={clearLogRecords}>
                清空日志浏览器
              </NButton>
            </div>
          </NDialog>
        </NModal>
      )
  },
})
