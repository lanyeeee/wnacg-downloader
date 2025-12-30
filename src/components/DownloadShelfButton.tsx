import { computed, defineComponent, onMounted, ref } from 'vue'
import { useStore } from '../store'
import { commands, events } from '../bindings'
import { MessageReactive, NButton, NPopconfirm, useMessage } from 'naive-ui'

export default defineComponent({
  name: 'DownloadShelfButton',
  props: {
    shelfId: {
      type: Number,
      required: true,
    },
  },
  setup(props) {
    const store = useStore()

    const message = useMessage()

    const popConfirmShowing = ref<boolean>(false)

    const rejectCooldown = ref<number>(0)
    const rejectButtonDisabled = computed(() => rejectCooldown.value > 0)

    const countdownInterval = ref<ReturnType<typeof setInterval>>(setInterval(() => {}, 1000))

    let downloadShelfMessage: MessageReactive | undefined

    onMounted(async () => {
      await events.downloadShelfEvent.listen(({ payload }) => {
        if (payload.event === 'GettingShelfComics') {
          downloadShelfMessage = message.loading('正在获取书架中的漫画', { duration: 0 })
        } else if (payload.event === 'CreatingDownloadTask' && downloadShelfMessage !== undefined) {
          const { current, total } = payload.data
          downloadShelfMessage.content = `正在创建下载任务(${current}/${total})`
        } else if (payload.event === 'End' && downloadShelfMessage !== undefined) {
          downloadShelfMessage.type = 'success'
          downloadShelfMessage.content = '为书架中的漫画创建下载任务成功'
          setTimeout(() => {
            downloadShelfMessage?.destroy()
            downloadShelfMessage = undefined
          }, 3000)
        }
      })
    })

    async function agree() {
      if (store.config === undefined) {
        return
      }

      store.config.imgDownloadIntervalSec = Math.max(1, Math.floor(store.config.imgConcurrency / 5))
      store.config.comicDownloadIntervalSec = Math.min(10, Math.floor(store.config.imgConcurrency * 3))

      popConfirmShowing.value = false

      const result = await commands.downloadShelf(props.shelfId)
      if (result.status === 'error') {
        console.error(result.error)
        downloadShelfMessage?.destroy()
        return
      }
    }

    async function reject() {
      popConfirmShowing.value = false
      const result = await commands.downloadShelf(props.shelfId)
      if (result.status === 'error') {
        console.error(result.error)
        downloadShelfMessage?.destroy()
        return
      }
    }

    function handleDownloadClick() {
      // 清理可能存在的旧计时器
      if (countdownInterval.value) {
        clearInterval(countdownInterval.value)
      }
      rejectCooldown.value = 10

      countdownInterval.value = setInterval(() => {
        rejectCooldown.value -= 1
        if (rejectCooldown.value <= 0) {
          clearInterval(countdownInterval.value)
        }
      }, 1000)
    }

    return () => (
      <NPopconfirm
        positiveText={null}
        negativeText={null}
        show={popConfirmShowing.value}
        onUpdate:show={(value) => (popConfirmShowing.value = value)}>
        {{
          default: () => (
            <div class="flex flex-col">
              <div>下载整个书架是个大任务</div>
              <div>为了减轻绅士漫画服务器压力</div>
              <div>将自动调整配置中的下载间隔</div>
              <div>
                <span>之后你随时可以在右上角的</span>
                <span class="bg-gray-2 px-1">配置</span>
                <span>调整</span>
              </div>
            </div>
          ),
          action: () => (
            <>
              <NButton size="small" disabled={rejectButtonDisabled.value} onClick={reject}>
                {rejectButtonDisabled.value && <span>不调整直接下载 ({rejectCooldown.value})</span>}
                {!rejectButtonDisabled.value && <span>不调整直接下载</span>}
              </NButton>
              <NButton size="small" type="primary" onClick={agree}>
                调整并下载
              </NButton>
            </>
          ),
          trigger: () => (
            <NButton type="primary" size="small" onClick={handleDownloadClick}>
              下载整个书架
            </NButton>
          ),
        }}
      </NPopconfirm>
    )
  },
})
