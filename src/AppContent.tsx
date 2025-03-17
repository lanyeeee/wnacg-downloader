import { defineComponent, onMounted, ref, watch } from 'vue'
import { useStore } from './store.ts'
import { commands } from './bindings.ts'
import LogViewer from './components/LogViewer.tsx'
import { notification, message, Button, Input, Avatar, Tabs } from 'ant-design-vue'
import LoginDialog from './components/LoginDialog.tsx'
import AboutDialog from './components/AboutDialog.tsx'
import DownloadingPane from './panes/DownloadingPane.tsx'
import SearchPane from './panes/SearchPane.tsx'
import ComicPane from './panes/ComicPane.tsx'
import FavoritePane from './panes/FavoritePane.tsx'
import DownloadedPane from './panes/DownloadedPane.tsx'
import { CurrentTabName } from './types.ts'

export default defineComponent({
  name: 'AppContent',
  setup() {
    const store = useStore()

    notification.config({ placement: 'bottomRight' })

    const logViewerShowing = ref<boolean>(false)
    const loginDialogShowing = ref<boolean>(false)
    const aboutDialogShowing = ref<boolean>(false)

    const searchPane = ref<InstanceType<typeof SearchPane>>()

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

    watch(
      () => store.config?.cookie,
      async () => {
        if (store.config?.cookie === '') {
          return
        }

        const result = await commands.getUserProfile()
        if (result.status === 'error') {
          console.error(result.error)
          store.userProfile = undefined
          return
        }

        store.userProfile = result.data
        message.success('获取用户信息成功')
      },
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
      <div class="h-screen flex flex-col">
        <div class="flex">
          <Input
            addonBefore="Cookie"
            value={store.config?.cookie}
            onUpdate:value={(value) => {
              if (store.config) {
                store.config.cookie = value
              }
            }}
            allowClear
          />
          <Button type="primary" onClick={() => (loginDialogShowing.value = true)}>
            账号登录
          </Button>
          <Button onClick={() => (logViewerShowing.value = true)}>日志</Button>
          <Button onClick={() => (aboutDialogShowing.value = true)}>关于</Button>
          <Button onClick={test}>测试用</Button>
          {store.userProfile && (
            <div class="flex items-center">
              <Avatar src={store.userProfile.avatar} />
              <span class="whitespace-nowrap">{store.userProfile.username}</span>
            </div>
          )}
          <LoginDialog
            showing={loginDialogShowing.value}
            onUpdate:showing={(showing) => (loginDialogShowing.value = showing)}
          />
          <LogViewer
            showing={logViewerShowing.value}
            onUpdate:showing={(showing) => (logViewerShowing.value = showing)}
          />
          <AboutDialog
            showing={aboutDialogShowing.value}
            onUpdate:showing={(showing) => (aboutDialogShowing.value = showing)}
          />
        </div>
        <div class="flex flex-1 overflow-hidden">
          <Tabs
            size="small"
            class="h-full basis-1/2"
            activeKey={store.currentTabName}
            onUpdate:activeKey={(key) => (store.currentTabName = key as CurrentTabName)}>
            <Tabs.TabPane key="search" tab="漫画搜索" class="overflow-auto">
              <SearchPane ref={searchPane} />
            </Tabs.TabPane>
            <Tabs.TabPane key="favorite" tab="我的书架" class="overflow-auto">
              <FavoritePane />
            </Tabs.TabPane>
            <Tabs.TabPane key="downloaded" tab="本地库存" class="overflow-auto">
              <DownloadedPane />
            </Tabs.TabPane>
            <Tabs.TabPane key="comic" tab="漫画详情" class="overflow-auto">
              {searchPane.value && <ComicPane searchByTag={searchPane.value.searchByTag} />}
            </Tabs.TabPane>
          </Tabs>
          <DownloadingPane class="h-full basis-1/2 overflow-auto" />
        </div>
      </div>
    )
  },
})
