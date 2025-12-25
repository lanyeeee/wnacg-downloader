import { defineComponent, onMounted, ref } from 'vue'
import { useStore } from '../../store.ts'
import { commands, events } from '../../bindings.ts'
import { open } from '@tauri-apps/plugin-dialog'
import { NButton, NIcon, NInput, NInputGroup, NInputGroupLabel, NTabPane, NTabs } from 'naive-ui'
import UncompletedProgresses from './components/UncompletedProgresses.tsx'
import CompletedProgress from './components/CompletedProgress.tsx'
import styles from './ProgressPane.module.css'
import { PhFolderOpen } from '@phosphor-icons/vue'

export default defineComponent({
  name: 'ProgressPane',
  setup() {
    const store = useStore()

    const downloadSpeed = ref<string>('')

    type TabName = 'uncompleted' | 'completed'
    const tabName = ref<TabName>('uncompleted')

    onMounted(async () => {
      await events.downloadSpeedEvent.listen(async ({ payload: { speed } }) => {
        downloadSpeed.value = speed
      })

      await events.downloadSleepingEvent.listen(async ({ payload: { comicId, remainingSec } }) => {
        const progressData = store.progresses.get(comicId)
        if (progressData !== undefined) {
          progressData.indicator = `将在${remainingSec}秒后继续下载`
        }
      })

      await events.downloadTaskEvent.listen(({ payload: downloadTaskEvent }) => {
        const { state, comic, downloadedImgCount, totalImgCount } = downloadTaskEvent

        if (state === 'Completed') {
          comic.isDownloaded = true
          if (store.getShelfResult !== undefined) {
            const completedResult = store.getShelfResult.comics.find(
              (comic) => comic.id === downloadTaskEvent.comic.id,
            )
            if (completedResult !== undefined) {
              completedResult.isDownloaded = true
            }
          }
          if (store.searchResult !== undefined) {
            const completedResult = store.searchResult.comics.find((comic) => comic.id === downloadTaskEvent.comic.id)
            if (completedResult !== undefined) {
              completedResult.isDownloaded = true
            }
          }
        }

        const percentage = (downloadedImgCount / totalImgCount) * 100

        let indicator = ''
        if (state === 'Pending') {
          indicator = `排队中`
        } else if (state === 'Downloading') {
          indicator = `下载中`
        } else if (state === 'Paused') {
          indicator = `已暂停`
        } else if (state === 'Cancelled') {
          indicator = `已取消`
        } else if (state === 'Completed') {
          indicator = `下载完成`
        } else if (state === 'Failed') {
          indicator = `下载失败`
        }
        if (totalImgCount !== 0) {
          indicator += ` ${downloadedImgCount}/${totalImgCount}`
        }

        const progressData = { ...downloadTaskEvent, percentage, indicator }
        store.progresses.set(comic.id, progressData)
      })
    })

    // 通过对话框选择下载目录
    async function selectDownloadDir() {
      if (store.config === undefined) {
        return
      }

      const selectedDirPath = await open({ directory: true })
      if (selectedDirPath === null) {
        return
      }
      store.config.downloadDir = selectedDirPath
    }

    async function showDownloadDirInFileManager() {
      if (store.config === undefined) {
        return
      }

      const result = await commands.showPathInFileManager(store.config.downloadDir)
      if (result.status === 'error') {
        console.error(result.error)
      }
    }

    return () => (
      <div class="flex flex-col h-full overflow-auto">
        <div class="flex box-border px-2 pt-2">
          <NInputGroup>
            <NInputGroupLabel size="small">下载目录</NInputGroupLabel>
            <NInput
              size="small"
              readonly
              value={store.config?.downloadDir}
              onUpdate:value={(value) => {
                if (store.config) {
                  store.config.downloadDir = value
                }
              }}
              // 如果直接用 onClick={selectDownloadDir}，运行没问题，但是ts会报错
              // 在vue里用jsx总有类似的狗屎问题 https://github.com/vuejs/babel-plugin-jsx/issues/555
              {...{
                onClick: selectDownloadDir,
              }}
            />
            <NButton class="w-10" size="small" onClick={showDownloadDirInFileManager}>
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
        <NTabs
          size="small"
          type="line"
          value={tabName.value}
          onUpdate:value={(value) => (tabName.value = value as TabName)}
          class={[`${styles.progressesTabs}`, 'flex-1 overflow-hidden pt-2']}>
          {{
            default: () => (
              <>
                <NTabPane class="h-full p-0! overflow-auto" name="uncompleted" tab="未完成">
                  <UncompletedProgresses />
                </NTabPane>
                <NTabPane class="h-full p-0! overflow-auto" name="completed" tab="已完成">
                  <CompletedProgress />
                </NTabPane>
              </>
            ),
            suffix: () => <span class="whitespace-nowrap text-ellipsis overflow-hidden">{downloadSpeed.value}</span>,
          }}
        </NTabs>
      </div>
    )
  },
})
