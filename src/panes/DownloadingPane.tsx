import { defineComponent, onMounted, ref } from 'vue'
import { useStore } from '../store.ts'
import { commands, events } from '../bindings.ts'
import { open } from '@tauri-apps/plugin-dialog'
import { Button, Input, Tabs } from 'ant-design-vue'
import UncompletedProgresses from '../components/UncompletedProgresses.tsx'
import CompletedProgress from '../components/CompletedProgress.tsx'
import styles from '../styles/DownloadingPane.module.css'

export default defineComponent({
  name: 'DownloadingPane',
  setup() {
    const store = useStore()

    const downloadSpeed = ref<string>('')

    onMounted(async () => {
      // 监听下载事件
      await events.downloadSpeedEvent.listen(async ({ payload: { speed } }) => {
        downloadSpeed.value = speed
      })

      await events.downloadTaskEvent.listen(({ payload: downloadTaskEvent }) => {
        const { state, comic, downloadedImgCount, totalImgCount } = downloadTaskEvent

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
          comic.isDownloaded = true
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
      <div class="flex flex-col h-full">
        <div class="flex h-9.5 items-center">
          <span class="text-lg font-bold">下载列表</span>
          <span class="ml-auto">下载速度: {downloadSpeed.value}</span>
        </div>
        <div class="flex">
          <Input
            size="small"
            addonBefore="下载目录"
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
          <Button size="small" onClick={showDownloadDirInFileManager}>
            打开目录
          </Button>
        </div>
        <Tabs size="small" class={`${styles.tabs} flex-1 overflow-hidden`}>
          <Tabs.TabPane key="uncompleted" tab="未完成" class="h-full overflow-auto">
            <UncompletedProgresses />
          </Tabs.TabPane>
          <Tabs.TabPane key="completed" tab="已完成" class="h-full overflow-auto">
            <CompletedProgress />
          </Tabs.TabPane>
        </Tabs>
      </div>
    )
  },
})
