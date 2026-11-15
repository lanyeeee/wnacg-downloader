import { defineComponent, ref } from 'vue'
import { useMessage, NModal, NDialog, NTabs, NTabPane, NInput, NAlert, NButton } from 'naive-ui'
import { useStore } from '../store.ts'
import { commands } from '../bindings.ts'
import FloatLabelInput from '../components/FloatLabelInput.tsx'

export default defineComponent({
  name: 'LoginDialog',
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

    const username = ref<string>('')
    const password = ref<string>('')
    const rawCookie = ref<string>('')
    const loading = ref<boolean>(false)
    const tab = ref<'account' | 'cookie'>('account')

    async function login() {
      if (store.config === undefined || loading.value) {
        return
      }
      if (username.value === '') {
        message.error('请输入用户名')
        return
      }
      if (password.value === '') {
        message.error('请输入密码')
        return
      }
      loading.value = true
      try {
        // 后端会自动识别滑块缺口、生成拖动轨迹并带上一次性通行证
        const result = await commands.login(username.value, password.value)
        if (result.status === 'error') {
          message.error(result.error.message)
          return
        }
        store.config.cookie = result.data
        message.success('登录成功')
        emit('update:showing', false)
      } finally {
        loading.value = false
      }
    }

    async function loginByCookie() {
      if (store.config === undefined || loading.value) {
        return
      }
      if (rawCookie.value.trim() === '') {
        message.error('请粘贴 cookie')
        return
      }
      loading.value = true
      try {
        const result = await commands.loginByCookie(rawCookie.value)
        if (result.status === 'error') {
          message.error(result.error.message)
          return
        }
        store.config.cookie = result.data
        message.success('cookie 校验通过，已登录')
        emit('update:showing', false)
      } finally {
        loading.value = false
      }
    }

    function handleKeydown(e: KeyboardEvent) {
      if (e.key === 'Enter' && tab.value === 'account') {
        login()
      }
    }

    return () => (
      <NModal show={props.showing} onUpdate:show={(value) => emit('update:showing', value)}>
        <NDialog
          showIcon={false}
          title="账号登录"
          positiveText={tab.value === 'account' ? '登录' : '导入 Cookie'}
          loading={loading.value}
          onPositiveClick={() => (tab.value === 'account' ? login() : loginByCookie())}
          onClose={() => emit('update:showing', false)}
          {...{ onKeydown: handleKeydown }}>
          <NTabs type="segment" value={tab.value} onUpdate:value={(value) => (tab.value = value as 'account' | 'cookie')}>
            <NTabPane name="account" tab="账号密码">
              <div class="flex flex-col gap-2">
                <FloatLabelInput
                  label="用户名"
                  value={username.value}
                  onUpdate:value={(value) => (username.value = value)}
                />
                <FloatLabelInput
                  label="密码"
                  value={password.value}
                  onUpdate:value={(value) => (password.value = value)}
                  type="password"
                />
                <NAlert type="info" showIcon={false}>
                  登录时会自动完成站点的滑块拼图验证，不需要手动拖动
                </NAlert>
              </div>
            </NTabPane>
            <NTabPane name="cookie" tab="Cookie 登录">
              <div class="flex flex-col gap-2">
                <NInput
                  type="textarea"
                  value={rawCookie.value}
                  onUpdate:value={(value) => (rawCookie.value = value)}
                  placeholder="支持：浏览器扩展导出的 JSON、Netscape cookies.txt、或 name=value; name2=value2"
                  autosize={{ minRows: 4, maxRows: 10 }}
                />
                <NAlert type="info" showIcon={false}>
                  在浏览器登录 https://wnacg.com 后，用 Cookie-Editor 等扩展导出 cookie（JSON 或 Netscape 格式）粘贴到这里即可
                </NAlert>
                <NButton
                  size="small"
                  onClick={async () => {
                    rawCookie.value = await commands.normalizeCookie(rawCookie.value)
                    message.success('已规范化')
                  }}>
                  规范化格式
                </NButton>
              </div>
            </NTabPane>
          </NTabs>
        </NDialog>
      </NModal>
    )
  },
})
