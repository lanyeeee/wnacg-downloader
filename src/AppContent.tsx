import { defineComponent, onMounted, ref, watch } from 'vue'
import { useStore } from './store.ts'
import { commands } from './bindings.ts'
import LogDialog from './dialogs/LogDialog.tsx'
import {
  useNotification,
  useMessage,
  NButton,
  NInput,
  NAvatar,
  NTabs,
  NTabPane,
  NInputGroup,
  NInputGroupLabel,
  NIcon,
} from 'naive-ui'
import LoginDialog from './dialogs/LoginDialog.tsx'
import AboutDialog from './dialogs/AboutDialog.tsx'
import ProgressPane from './panes/ProgressPane/ProgressPane.tsx'
import SearchPane from './panes/SearchPane.tsx'
import ComicPane from './panes/ComicPane.tsx'
import ShelfPane from './panes/ShelfPane.tsx'
import DownloadedPane from './panes/DownloadedPane/DownloadedPane.tsx'
import { CurrentTabName } from './types.ts'
import { PhClockCounterClockwise, PhGearSix, PhInfo, PhUser } from '@phosphor-icons/vue'
import SettingsDialog from './dialogs/SettingsDialog.tsx'

export default defineComponent({
  name: 'AppContent',
  setup() {
    const store = useStore()

    const notification = useNotification()
    const message = useMessage()

    const logDialogShowing = ref<boolean>(false)
    const loginDialogShowing = ref<boolean>(false)
    const settingsDialogShowing = ref<boolean>(false)
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
      async (value, oldValue) => {
        if (store.config === undefined) {
          return
        }
        if (oldValue !== undefined && oldValue !== '' && value === '') {
          // 如果旧的 cookie 不为空，新的 cookie 为空，相当于退出登录
          store.userProfile = undefined
          store.config.cookie = ''
          message.success('已退出登录')
          return
        } else if (value === undefined || value === '') {
          // 如果 cookie 为空，说明用户没有登录
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
          title: '日志目录大小超过50MB，请及时清理日志文件',
          description: () => (
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

    return () =>
      store.config !== undefined && (
        <div class="h-screen flex flex-col">
          <div class="flex gap-1 pt-2 px-2">
            <NInputGroup>
              <NInputGroupLabel>Cookie</NInputGroupLabel>
              <NInput
                value={store.config?.cookie}
                onUpdate:value={(value) => {
                  if (store.config) {
                    store.config.cookie = value
                  }
                }}
                placeholder="手动输入或点击右侧的按钮登录"
                clearable
              />
              <NButton type="primary" onClick={() => (loginDialogShowing.value = true)}>
                {{
                  icon: () => (
                    <NIcon size={20}>
                      <PhUser />
                    </NIcon>
                  ),
                  default: () => <div>登录</div>,
                }}
              </NButton>
            </NInputGroup>

            {store.userProfile && (
              <div class="flex items-center">
                <NAvatar src={store.userProfile.avatar} round />
                <span class="whitespace-nowrap">{store.userProfile.username}</span>
              </div>
            )}
          </div>

          <div class="flex flex-1 overflow-hidden">
            <NTabs
              class="h-full w-1/2"
              value={store.currentTabName}
              onUpdate:value={(value) => (store.currentTabName = value as CurrentTabName)}
              type="line"
              size="small"
              animated>
              <NTabPane class="h-full overflow-auto p-0!" name="search" tab="漫画搜索" display-directive="show">
                <SearchPane ref={searchPane} />
              </NTabPane>
              <NTabPane class="h-full overflow-auto p-0!" name="shelf" tab="我的书架" display-directive="show">
                <ShelfPane />
              </NTabPane>
              <NTabPane class="h-full overflow-auto p-0!" name="downloaded" tab="本地库存" display-directive="show">
                <DownloadedPane />
              </NTabPane>
              <NTabPane class="h-full overflow-auto p-0!" name="comic" tab="漫画详情" display-directive="show">
                {searchPane.value && <ComicPane searchByTag={searchPane.value.searchByTag} />}
              </NTabPane>
            </NTabs>

            <div class="w-1/2 overflow-auto flex flex-col">
              <div class="flex min-h-8.5 gap-col-1 mx-2 items-center border-solid border-0 border-b box-border border-[rgb(239,239,245)]">
                <div class="text-xl font-bold box-border">下载列表</div>
                <NButton class="ml-auto" size="small" onClick={() => (logDialogShowing.value = true)}>
                  {{
                    icon: () => (
                      <NIcon size={20}>
                        <PhClockCounterClockwise />
                      </NIcon>
                    ),
                    default: () => <div>日志</div>,
                  }}
                </NButton>
                <NButton size="small" onClick={() => (settingsDialogShowing.value = true)}>
                  {{
                    icon: () => (
                      <NIcon size={20}>
                        <PhGearSix />
                      </NIcon>
                    ),
                    default: () => <div>配置</div>,
                  }}
                </NButton>
                <NButton size="small" onClick={() => (aboutDialogShowing.value = true)}>
                  {{
                    icon: () => (
                      <NIcon size={20}>
                        <PhInfo />
                      </NIcon>
                    ),
                    default: () => <div>关于</div>,
                  }}
                </NButton>
              </div>
              <ProgressPane />
            </div>
            <LoginDialog
              showing={loginDialogShowing.value}
              onUpdate:showing={(showing) => (loginDialogShowing.value = showing)}
            />
            <LogDialog
              showing={logDialogShowing.value}
              onUpdate:showing={(showing) => (logDialogShowing.value = showing)}
            />
            <SettingsDialog
              showing={settingsDialogShowing.value}
              onUpdate:showing={(showing) => (settingsDialogShowing.value = showing)}
            />
            <AboutDialog
              showing={aboutDialogShowing.value}
              onUpdate:showing={(showing) => (aboutDialogShowing.value = showing)}
            />
          </div>
        </div>
      )
  },
})
