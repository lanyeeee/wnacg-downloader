import { computed, defineComponent, onMounted, ref, watch } from 'vue'
import { notification, Modal, Input, Select, Button, Checkbox } from 'ant-design-vue'
import { commands, events, LogEvent, LogLevel } from '../bindings.ts'
import { appDataDir } from '@tauri-apps/api/path'
import { path } from '@tauri-apps/api'
import { useStore } from '../store.ts'

type LogRecord = LogEvent & { id: number; formatedLog: string }

export default defineComponent({
  name: 'LogViewer',
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

    const logRecords = ref<LogRecord[]>([])
    const searchText = ref<string>('')
    const selectedLevel = ref<LogLevel>('INFO')
    const logsDirSize = ref<number>(0)
    let nextLogRecordId = 1

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
        if (!showing) {
          return
        }

        const result = await commands.getLogsDirSize()
        if (result.status === 'error') {
          console.error(result.error)
          return
        }
        logsDirSize.value = result.data
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
            message: fields['err_title'] as string,
            description: fields['message'] as string,
            duration: 0,
          })
        }
      })
    })

    function clearLogRecords() {
      logRecords.value = []
      nextLogRecordId = 1
    }

    return () => (
      <Modal
        title={<div class="flex items-center">日志目录总大小：{formatedLogsDirSize.value}</div>}
        open={props.showing}
        onCancel={() => emit('update:showing', false)}
        width="95%"
        footer={null}>
        <div class="mb-2 flex flex-wrap gap-2">
          <Input
            class="w-40%"
            size="small"
            placeholder="搜索日志..."
            value={searchText.value}
            onChange={(e) => (searchText.value = e.target.value ?? '')}
            allowClear
          />
          <Select
            class="w-25"
            size="small"
            value={selectedLevel.value}
            onChange={(value) => (selectedLevel.value = value as LogLevel)}
            options={logLevelOptions}
          />
          <div class="flex flex-wrap gap-2 ml-auto">
            <Button size="small" onClick={showLogsDirInFileManager}>
              打开日志目录
            </Button>
            <Checkbox
              class="select-none"
              checked={store.config?.enableFileLogger}
              onChange={() => {
                if (store.config) {
                  store.config.enableFileLogger = !store.config.enableFileLogger
                }
              }}>
              输出文件日志
            </Checkbox>
          </div>
        </div>

        <div class="h-[calc(100vh-300px)] overflow-auto bg-gray-900 p-3">
          {filteredLogs.value.map(({ id, level, formatedLog }) => (
            <div key={id} class={`p-1 hover:bg-white/10 whitespace-pre-wrap ${getLevelStyles(level)}`}>
              {formatedLog}
            </div>
          ))}
        </div>
        <div class="pt-1 flex">
          <Button class="ml-auto" size="small" onClick={clearLogRecords} danger>
            清空日志浏览器
          </Button>
        </div>
      </Modal>
    )
  },
})

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

function formatLogEvent(logEvent: LogEvent): string {
  const { timestamp, level, fields, target, filename, line_number } = logEvent
  const fields_str = Object.entries(fields)
    .map(([key, value]) => `${key}=${value}`)
    .join(' ')
  return `${timestamp} ${level} ${target}: ${filename}:${line_number} ${fields_str}`
}

async function showLogsDirInFileManager() {
  const logsDir = await path.join(await appDataDir(), '日志')
  const result = await commands.showPathInFileManager(logsDir)
  if (result.status === 'error') {
    console.error(result.error)
  }
}
