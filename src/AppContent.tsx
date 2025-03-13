import { defineComponent, onMounted, ref, watch } from 'vue'
import { useStore } from './store.ts'
import { commands } from './bindings.ts'
import LogViewer from './components/LogViewer.tsx'
import { notification, message, Button } from 'ant-design-vue'

export default defineComponent({
  name: 'AppContent',
  setup() {
    const store = useStore()

    notification.config({ placement: 'bottomRight' })

    const logViewerShowing = ref<boolean>(false)

    watch(
      () => store.config,
      async () => {
        if (store.config === undefined) {
          return
        }
        await commands.saveConfig(store.config)
        message.success('保存配置成功')
      },
      { deep: true },
    )

    onMounted(async () => {
      // 屏蔽浏览器右键菜单
      document.oncontextmenu = (event) => {
        event.preventDefault()
      }
      // 获取配置
      store.config = await commands.getConfig()
      // 检查日志目录大小
      const result = await commands.getLogsDirSize()
      if (result.status === 'error') {
        console.error(result.error)
        return
      }
      if (result.data > 50 * 1024 * 1024) {
        notification.warning({
          message: '日志目录大小超过50MB，请及时清理日志文件',
          description: (
            <>
              <div>
                点击右上角的 <span class="bg-gray-2 px-1">查看日志</span> 按钮
              </div>
              <div>
                里边有 <span class="bg-gray-2 px-1">打开日志目录</span> 按钮
              </div>
              <div>
                你也可以在里边取消勾选 <span class="bg-gray-2 px-1">输出文件日志</span>
              </div>
              <div>这样将不再产生文件日志</div>
            </>
          ),
        })
      }
    })

    function test() {
      notification.error({
        message: 'messagemessagemessagemessagemessagemessagemessagemessagemessagemessagemessage',
        description:
          'descriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescriptiondescription',
        duration: 0,
      })
    }

    return () => (
      <div>
        <Button onClick={() => (logViewerShowing.value = true)}>查看日志</Button>
        <Button onClick={test}>测试用</Button>
        <LogViewer
          showing={logViewerShowing.value}
          onUpdate:showing={(showing) => (logViewerShowing.value = showing)}
        />
      </div>
    )
  },
})
