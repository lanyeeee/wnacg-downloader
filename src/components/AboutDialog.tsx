import { defineComponent, onMounted, ref } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { Modal, Typography } from 'ant-design-vue'
import icon from '../../src-tauri/icons/128x128.png'

export default defineComponent({
  name: 'AboutDialog',
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
    const version = ref<string>('')

    onMounted(async () => {
      version.value = await getVersion()
    })

    return () => (
      <Modal open={props.showing} onCancel={() => emit('update:showing', false)} footer={null}>
        <div class="flex flex-col items-center gap-row-6">
          <img src={icon} alt="icon" class="w-32 h-32" />
          <div class="text-center text-gray-400 text-xs">
            <div>
              如果本项目对你有帮助，欢迎来
              <Typography.Link href="https://github.com/lanyeeee/wnacg-downloader" target="_blank">
                GitHub
              </Typography.Link>
              点个Star⭐支持！
            </div>
            <div class="mt-1">你的支持是我持续更新维护的动力🙏</div>
          </div>
          <div class="flex flex-col w-full gap-row-3 px-6">
            <div class="flex items-center justify-between py-2 px-4 bg-gray-100 rounded-lg">
              <span class="text-gray-500">软件版本</span>
              <div class="font-medium">v{version.value}</div>
            </div>
            <div class="flex items-center justify-between py-2 px-4 bg-gray-100 rounded-lg">
              <span class="text-gray-500">开源地址</span>
              <Typography.Link href="https://github.com/lanyeeee/wnacg-downloader" target="_blank">
                GitHub
              </Typography.Link>
            </div>
            <div class="flex items-center justify-between py-2 px-4 bg-gray-100 rounded-lg">
              <span class="text-gray-500">问题反馈</span>
              <Typography.Link href="https://github.com/lanyeeee/wnacg-downloader/issues" target="_blank">
                GitHub Issues
              </Typography.Link>
            </div>
          </div>
          <div class="flex flex-col text-xs text-gray-400">
            <div>
              Copyright © 2025{' '}
              <Typography.Link href="https://github.com/lanyeeee" target="_blank">
                lanyeeee
              </Typography.Link>
            </div>
            <div>
              Released under{' '}
              <Typography.Link href="https://github.com/lanyeeee/wnacg-downloader/blob/main/LICENSE" target="_blank">
                MIT License
              </Typography.Link>
            </div>
          </div>
        </div>
      </Modal>
    )
  },
})
