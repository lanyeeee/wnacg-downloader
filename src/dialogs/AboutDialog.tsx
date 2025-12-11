import { defineComponent, onMounted, ref } from 'vue'
import { getVersion } from '@tauri-apps/api/app'
import { NModal, NDialog, NA } from 'naive-ui'
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
      <NModal show={props.showing} onUpdate:show={(value) => emit('update:showing', value)}>
        <NDialog showIcon={false} onClose={() => emit('update:showing', false)}>
          <div class="flex flex-col items-center gap-row-6">
            <img src={icon} alt="icon" class="w-32 h-32" />
            <div class="text-center text-gray-400 text-xs">
              <div>
                如果本项目对你有帮助，欢迎来
                <NA {...{ href: 'https://github.com/lanyeeee/wnacg-downloader', target: '_blank' }}>GitHub</NA>
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
                <NA {...{ href: 'https://github.com/lanyeeee/wnacg-downloader', target: '_blank' }}>GitHub</NA>
              </div>
              <div class="flex items-center justify-between py-2 px-4 bg-gray-100 rounded-lg">
                <span class="text-gray-500">问题反馈</span>
                <NA {...{ href: 'https://github.com/lanyeeee/wnacg-downloader/issues', target: '_blank' }}>
                  GitHub Issues
                </NA>
              </div>
            </div>
            <div class="flex flex-col text-xs text-gray-400">
              <div>
                Copyright © 2025 <NA {...{ href: 'https://github.com/lanyeeee', target: '_blank' }}>lanyeeee</NA>
              </div>
              <div>
                Released under{' '}
                <NA {...{ href: 'https://github.com/lanyeeee/wnacg-downloader/blob/main/LICENSE', target: '_blank' }}>
                  MIT License
                </NA>
              </div>
            </div>
          </div>
        </NDialog>
      </NModal>
    )
  },
})
