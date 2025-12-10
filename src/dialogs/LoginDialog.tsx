import { defineComponent, ref } from 'vue'
import { Button, Input, message, Modal } from 'ant-design-vue'
import { useStore } from '../store.ts'
import { commands } from '../bindings.ts'

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

    const username = ref<string>('')
    const password = ref<string>('')

    async function login() {
      if (store.config === undefined) {
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

      const result = await commands.login(username.value, password.value)
      if (result.status === 'error') {
        console.error(result.error)
        return
      }

      message.success('登录成功')
      store.config.cookie = result.data
      emit('update:showing', false)
    }

    return () => (
      <Modal
        title="账号登录"
        open={props.showing}
        onCancel={() => emit('update:showing', false)}
        v-slots={{
          footer: () => (
            <Button type="primary" onClick={login}>
              登录
            </Button>
          ),
        }}>
        <Input
          addonBefore="用户名"
          value={username.value}
          onUpdate:value={(value) => (username.value = value)}
          onPressEnter={login}
        />
        <Input.Password
          v-slots={{
            addonBefore: () => <div class="mx-1.75">密码</div>,
          }}
          value={password.value}
          onUpdate:value={(value) => (password.value = value)}
          onPressEnter={login}
        />
      </Modal>
    )
  },
})
