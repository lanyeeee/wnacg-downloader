import { defineComponent } from 'vue'
import { useStore } from '../store.ts'
import { Button, InputNumber, Modal, Radio, RadioGroup, Tooltip, message } from 'ant-design-vue'
import { commands } from '../bindings.ts'
import { path } from '@tauri-apps/api'
import { appDataDir } from '@tauri-apps/api/path'

export default defineComponent({
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

    async function showConfigPathInFileManager() {
      const configPath = await path.join(await appDataDir(), 'config.json')
      const result = await commands.showPathInFileManager(configPath)
      if (result.status === 'error') {
        console.error(result.error)
      }
    }

    return () => (
      <Modal title="更多设置" open={props.showing} onCancel={() => emit('update:showing', false)} footer={false}>
        <div class="flex flex-col gap-row-2">
          <div>
            图片下载格式：
            <RadioGroup
              name="downloadFormat"
              value={store?.config?.downloadFormat}
              onUpdate:value={(value) => {
                if (store.config) {
                  store.config.downloadFormat = value
                }
              }}>
              <Tooltip
                placement="top"
                v-slots={{
                  title: () => (
                    <>
                      <div>当原图不为jpg时</div>
                      <div>会自动转换为jpg</div>
                    </>
                  ),
                }}>
                <Radio value="Jpeg">jpg</Radio>
              </Tooltip>
              <Tooltip
                placement="top"
                v-slots={{
                  title: () => (
                    <>
                      <div>当原图不为png时</div>
                      <div>会自动转换为png</div>
                    </>
                  ),
                }}>
                <Radio value="Png">png</Radio>
              </Tooltip>
              <Tooltip
                placement="top"
                v-slots={{
                  title: () => (
                    <>
                      <div>当原图不为webp时</div>
                      <div>会自动转换为webp</div>
                    </>
                  ),
                }}>
                <Radio value="Webp">webp</Radio>
              </Tooltip>
              <Tooltip
                placement="top"
                v-slots={{
                  title: () => (
                    <>
                      <div>保持原图格式，不做任何转换</div>
                      <div class="text-red">不支持断点续传</div>
                    </>
                  ),
                }}>
                <Radio value="Original">原始格式</Radio>
              </Tooltip>
            </RadioGroup>
          </div>
          <div class="flex gap-1">
            <InputNumber
              size="small"
              min={1}
              addonBefore="漫画并发数"
              value={store.config?.comicConcurrency}
              onUpdate:value={async (value) => {
                if (store.config) {
                  message.warning('对漫画并发数的修改需要重启才能生效')
                  store.config.comicConcurrency = value as number
                }
              }}
            />
          </div>
          <div class="flex gap-1">
            <InputNumber
              size="small"
              min={1}
              addonBefore="图片并发数"
              value={store.config?.imgConcurrency}
              onUpdate:value={async (value) => {
                if (store.config) {
                  message.warning('对图片并发数的修改需要重启才能生效')
                  store.config.imgConcurrency = value as number
                }
              }}
            />
          </div>
        </div>
        <div class="flex justify-end mt-4">
          <Button size="small" onClick={showConfigPathInFileManager}>
            打开配置目录
          </Button>
        </div>
      </Modal>
    )
  },
})
