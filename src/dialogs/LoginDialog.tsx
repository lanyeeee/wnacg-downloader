import { defineComponent, onBeforeUnmount, ref, watch } from 'vue'
import { useMessage, NModal, NDialog } from 'naive-ui'
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

    const loggingIn = ref(false)
    const username = ref('')
    const password = ref('')

    async function login() {
      if (loggingIn.value || store.config === undefined) return
      if (!username.value.trim()) {
        message.error('请输入用户名')
        return
      }
      if (!password.value) {
        message.error('请输入密码')
        return
      }
      loggingIn.value = true
      try {
        const result = await commands.login(username.value.trim(), password.value)
        if (result.status === 'error') {
          message.error(`${result.error.err_title}：${result.error.err_message}`)
          return
        }
        if (result.data !== null) {
          store.config.cookie = result.data
          password.value = ''
          message.success('登录成功')
          emit('update:showing', false)
        }
      } catch {
        message.error('登录失败，请重试')
      } finally {
        loggingIn.value = false
      }
    }

    function close() {
      password.value = ''
      emit('update:showing', false)
    }

    watch(
      () => props.showing,
      (showing) => {
        if (!showing) {
          void commands.closeLoginWindow()
        }
      },
    )
    onBeforeUnmount(() => void commands.closeLoginWindow())

    return () => (
      <NModal show={props.showing} maskClosable={false} onUpdate:show={close}>
        <NDialog
          showIcon={false}
          title="账号登录"
          positiveText={loggingIn.value ? '登录中…' : '登录'}
          positiveButtonProps={{ loading: loggingIn.value, disabled: loggingIn.value }}
          negativeText="取消"
          onPositiveClick={login}
          onNegativeClick={close}
          onClose={close}
          {...{
            onKeydown: (event: KeyboardEvent) => {
              if (event.key === 'Enter') void login()
            },
          }}>
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
            {loggingIn.value && (
              <span class="text-sm text-gray-500">如需验证，请在弹出的小窗中完成滑块，随后会自动继续登录。</span>
            )}
          </div>
        </NDialog>
      </NModal>
    )
  },
})
