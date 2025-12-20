import { computed, defineComponent, onMounted, ref, watch } from 'vue'
import {
  NButton,
  NInput,
  NInputGroup,
  NInputGroupLabel,
  useMessage,
  NPagination,
  MessageReactive,
  NIcon,
} from 'naive-ui'
import { open } from '@tauri-apps/plugin-dialog'
import { useStore } from '../../store.ts'
import { Comic, commands, events } from '../../bindings.ts'
import DownloadedComicCard from './components/DownloadedComicCard.tsx'
import { PhFolderOpen } from '@phosphor-icons/vue'

interface ProgressData {
  title: string
  progressMessage: MessageReactive
}

export default defineComponent({
  name: 'DownloadedPane',
  setup() {
    const store = useStore()

    const message = useMessage()

    const comicCardContainer = ref<HTMLElement>()

    const PAGE_SIZE = 20
    // 已下载的漫画
    const downloadedComics = ref<Comic[]>([])
    // 当前页码
    const currentPage = ref<number>(1)
    // 总页数
    const pageCount = computed<number>(() => {
      if (downloadedComics.value.length === 0) {
        return 1
      }
      return Math.ceil(downloadedComics.value.length / PAGE_SIZE)
    })
    // 当前页的漫画
    const currentPageComics = computed(() => {
      const start = (currentPage.value - 1) * PAGE_SIZE
      const end = start + PAGE_SIZE
      return downloadedComics.value.slice(start, end)
    })

    watch(currentPage, () => {
      if (comicCardContainer.value !== undefined) {
        comicCardContainer.value.scrollTo({ top: 0, behavior: 'instant' })
      }
    })

    // 监听标签页变化，更新下载的漫画列表
    watch(
      () => store.currentTabName,
      async () => {
        if (store.currentTabName !== 'downloaded') {
          return
        }

        const result = await commands.getDownloadedComics()
        if (result.status === 'error') {
          console.error(result.error)
          return
        }
        downloadedComics.value = result.data
      },
      { immediate: true },
    )

    const progresses = new Map<string, ProgressData>()
    onMounted(async () => {
      await events.exportCbzEvent.listen(async ({ payload: exportCbzEvent }) => {
        if (exportCbzEvent.event === 'Start') {
          const { uuid, title } = exportCbzEvent.data
          progresses.set(uuid, { title, progressMessage: message.loading(`${title} 正在导出cbz`, { duration: 0 }) })
        } else if (exportCbzEvent.event === 'End') {
          const { uuid } = exportCbzEvent.data
          const progressData = progresses.get(uuid)
          if (progressData === undefined) {
            return
          }
          progressData.progressMessage.type = 'success'
          progressData.progressMessage.content = `${progressData.title} 导出cbz完成`
          setTimeout(() => {
            progressData.progressMessage.destroy()
            progresses.delete(uuid)
          }, 3000)
        }
      })

      await events.exportPdfEvent.listen(async ({ payload: exportPdfEvent }) => {
        if (exportPdfEvent.event === 'Start') {
          const { uuid, title } = exportPdfEvent.data
          progresses.set(uuid, { title, progressMessage: message.loading(`${title} 正在导出pdf`, { duration: 0 }) })
        } else if (exportPdfEvent.event === 'End') {
          const { uuid } = exportPdfEvent.data
          const progressData = progresses.get(uuid)
          if (progressData === undefined) {
            return
          }
          progressData.progressMessage.type = 'success'
          progressData.progressMessage.content = `${progressData.title} 导出pdf完成`
          setTimeout(() => {
            progressData.progressMessage.destroy()
            progresses.delete(uuid)
          }, 3000)
        }
      })
    })

    // 选择导出目录
    async function selectExportDir() {
      if (store.config === undefined) {
        return
      }

      const selectedDirPath = await open({ directory: true })
      if (selectedDirPath === null) {
        return
      }
      store.config.exportDir = selectedDirPath
    }

    async function showExportDirInFileManager() {
      if (store.config === undefined) {
        return
      }

      const result = await commands.showPathInFileManager(store.config.exportDir)
      if (result.status === 'error') {
        console.error(result.error)
      }
    }

    return () => (
      <div class="h-full flex flex-col">
        <div class="flex px-2 pt-2">
          <NInputGroup>
            <NInputGroupLabel size="small">导出目录</NInputGroupLabel>
            <NInput
              size="small"
              readonly
              value={store.config?.exportDir}
              onUpdate:value={(value) => {
                if (store.config) {
                  store.config.exportDir = value
                }
              }}
              // 如果直接用 onClick={selectExportDir}，运行没问题，但是ts会报错
              // 在vue里用jsx总有类似的狗屎问题 https://github.com/vuejs/babel-plugin-jsx/issues/555
              {...{
                onClick: selectExportDir,
              }}
            />
            <NButton class="w-10" size="small" onClick={showExportDirInFileManager}>
              {{
                icon: () => (
                  <NIcon size={20}>
                    <PhFolderOpen />
                  </NIcon>
                ),
              }}
            </NButton>
          </NInputGroup>
        </div>
        <div class="flex flex-col overflow-auto">
          <div ref={comicCardContainer} class="flex flex-col gap-row-2 overflow-auto p-2">
            {currentPageComics.value.map((comic) => (
              <DownloadedComicCard key={comic.id} comic={comic} />
            ))}
          </div>
        </div>
        <NPagination
          class="p-2 mt-auto"
          page={currentPage.value}
          pageCount={pageCount.value}
          onUpdate:page={(page) => (currentPage.value = page)}
        />
      </div>
    )
  },
})
