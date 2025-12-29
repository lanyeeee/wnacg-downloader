import { defineComponent, ref } from 'vue'
import { useMessage, NModal, NDialog, NInput } from 'naive-ui'
import { useStore } from '../store.ts'
import { commands } from '../bindings.ts'
import { extractComicId } from '../utils.ts'

export default defineComponent({
  name: 'BatchDownloadDialog',
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

    const message = useMessage()

    const inputString = ref<string>()

    async function confirm() {
      if (store.config === undefined) {
        return
      }

      const intervalMs = store.config.batchDownloadIntervalMs

      const lines = inputString.value?.split('\n')
      const comicIds = new Set(lines?.map(extractComicId).filter((comicId) => comicId !== undefined))
      if (comicIds.size === 0) {
        message.error('没有解析出任何漫画ID，请检查格式是否正确')
        return
      }

      const progresses = Array.from(store.progresses.entries())
      const uncompletedProgresses = new Map(
        progresses.filter(([, { state }]) => state !== 'Completed' && state !== 'Cancelled'),
      )

      emit('update:showing', false)

      const current = ref<number>(0)
      const total = comicIds.size
      const batchMessage = message.loading(() => `正在批量创建下载任务(${current.value}/${total})`, { duration: 0 })

      for (const comicId of comicIds) {
        current.value++

        if (uncompletedProgresses.has(comicId)) {
          continue
        }

        const getComicResult = await commands.getComic(comicId)
        if (getComicResult.status === 'error') {
          console.error(getComicResult.error)
          continue
        }

        const comic = getComicResult.data
        if (comic.isDownloaded === true) {
          await new Promise((resolve) => setTimeout(resolve, intervalMs))
          continue
        }

        await commands.createDownloadTask(comic)

        await new Promise((resolve) => setTimeout(resolve, intervalMs))
      }

      batchMessage.type = 'success'
      batchMessage.content = `批量下载任务创建结束(${current.value}/${total})`
      setTimeout(() => batchMessage.destroy(), 3000)
    }

    return () => (
      <NModal show={props.showing} onUpdate:show={(value) => emit('update:showing', value)}>
        <NDialog
          showIcon={false}
          title="批量下载"
          positiveText="确定"
          onPositiveClick={confirm}
          onClose={() => emit('update:showing', false)}>
          <NInput
            value={inputString.value}
            onUpdate:value={(value) => (inputString.value = value)}
            type="textarea"
            placeholder="漫画ID或链接，每行一个"
            autosize={{ minRows: 10, maxRows: 10 }}
          />
        </NDialog>
      </NModal>
    )
  },
})
