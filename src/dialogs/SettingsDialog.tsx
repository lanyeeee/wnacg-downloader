import { defineComponent, onMounted, ref } from 'vue'
import { useStore } from '../store.ts'
import {
  NButton,
  NCheckbox,
  NInputNumber,
  NModal,
  NDialog,
  NRadio,
  NRadioGroup,
  NTooltip,
  useMessage,
  NInputGroup,
  NInputGroupLabel,
  NRadioButton,
  NInput,
} from 'naive-ui'
import { commands } from '../bindings.ts'
import { path } from '@tauri-apps/api'
import { appDataDir } from '@tauri-apps/api/path'
import { openUrl } from '@tauri-apps/plugin-opener'

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
    const message = useMessage()

    const proxyHost = ref<string>('')
    const customApiDomain = ref<string>('')

    onMounted(() => {
      console.log(store.config)
      if (store.config !== undefined) {
        proxyHost.value = store.config.proxyHost
        customApiDomain.value = store.config.customApiDomain
      }
    })

    async function showConfigPathInFileManager() {
      const configPath = await path.join(await appDataDir(), 'config.json')
      const result = await commands.showPathInFileManager(configPath)
      if (result.status === 'error') {
        console.error(result.error)
      }
    }

    return () => (
      <NModal show={props.showing} onUpdate:show={(value) => emit('update:showing', value)}>
        <NDialog class="w-140!" showIcon={false} title="配置" onClose={() => emit('update:showing', false)}>
          <div class="flex flex-col">
            <span class="font-bold">下载格式</span>
            <NRadioGroup
              value={store?.config?.downloadFormat}
              onUpdate:value={(value) => {
                if (store.config) {
                  store.config.downloadFormat = value
                }
              }}>
              <NTooltip placement="top" trigger="hover">
                {{
                  trigger: () => <NRadio value="Jpeg">jpg</NRadio>,
                  default: () => (
                    <>
                      <div>当原图不为jpg时</div>
                      <div>会自动转换为jpg</div>
                    </>
                  ),
                }}
              </NTooltip>
              <NTooltip placement="top" trigger="hover">
                {{
                  trigger: () => <NRadio value="Png">png</NRadio>,
                  default: () => (
                    <>
                      <div>当原图不为png时</div>
                      <div>会自动转换为png</div>
                    </>
                  ),
                }}
              </NTooltip>
              <NTooltip placement="top" trigger="hover">
                {{
                  trigger: () => <NRadio value="Webp">webp</NRadio>,
                  default: () => (
                    <>
                      <div>当原图不为webp时</div>
                      <div>会自动转换为webp</div>
                    </>
                  ),
                }}
              </NTooltip>
              <NTooltip placement="top" trigger="hover">
                {{
                  trigger: () => <NRadio value="Original">原始格式</NRadio>,
                  default: () => (
                    <>
                      <div>保持原图格式，不做任何转换</div>
                      <div class="text-red">不支持断点续传</div>
                    </>
                  ),
                }}
              </NTooltip>
            </NRadioGroup>

            <span class="font-bold mt-2">API域名</span>
            <NRadioGroup
              size="small"
              value={store.config?.apiDomainMode}
              onUpdate:value={(value) => {
                if (store.config !== undefined) {
                  store.config.apiDomainMode = value
                }
              }}>
              <NRadioButton value="Default">默认</NRadioButton>
              <NRadioButton value="Custom">自定义</NRadioButton>
            </NRadioGroup>
            {store.config?.apiDomainMode === 'Custom' && (
              <NInputGroup class="mt-1">
                <NInputGroupLabel size="small">自定义API域名</NInputGroupLabel>
                <NInput
                  size="small"
                  placeholder=""
                  value={customApiDomain.value}
                  onUpdate:value={(value) => {
                    if (store.config !== undefined) {
                      customApiDomain.value = value
                    }
                  }}
                  onBlur={() => {
                    if (store.config !== undefined) {
                      store.config.customApiDomain = customApiDomain.value
                    }
                  }}
                  onKeydown={(e: KeyboardEvent) => {
                    console.log(e)
                    if (e.key === 'Enter' && store.config !== undefined) {
                      store.config.customApiDomain = customApiDomain.value
                    }
                  }}
                />
                <NButton size="small" onClick={() => openUrl('https://wn01.link/')}>
                  打开发布页
                </NButton>
              </NInputGroup>
            )}

            <span class="font-bold mt-2">下载速度</span>
            <div class="flex flex-col gap-1">
              <div class="flex gap-1">
                <NInputGroup class="w-35%">
                  <NInputGroupLabel size="small">漫画并发数</NInputGroupLabel>
                  <NInputNumber
                    class="w-full"
                    size="small"
                    value={store.config?.comicConcurrency}
                    onUpdate:value={(value) => {
                      if (store.config && value !== null) {
                        message.warning('对漫画并发数的修改需要重启才能生效')
                        store.config.comicConcurrency = value
                      }
                    }}
                    min={1}
                    parse={(x: string) => Number(x)}
                  />
                </NInputGroup>
                <NInputGroup class="w-65%">
                  <NInputGroupLabel size="small">每本漫画下载完成后休息</NInputGroupLabel>
                  <NInputNumber
                    class="w-full"
                    size="small"
                    value={store.config?.comicDownloadIntervalSec}
                    onUpdate:value={(value) => {
                      if (store.config && value !== null) {
                        store.config.comicDownloadIntervalSec = value
                      }
                    }}
                    min={0}
                    parse={(x: string) => Number(x)}
                  />
                  <NInputGroupLabel size="small">秒</NInputGroupLabel>
                </NInputGroup>
              </div>

              <div class="flex gap-1">
                <NInputGroup class="w-35%">
                  <NInputGroupLabel size="small">图片并发数</NInputGroupLabel>
                  <NInputNumber
                    class="w-full"
                    size="small"
                    value={store.config?.imgConcurrency}
                    onUpdate:value={(value) => {
                      if (store.config && value !== null) {
                        message.warning('对图片并发数的修改需要重启才能生效')
                        store.config.imgConcurrency = value
                      }
                    }}
                    min={1}
                    parse={(x: string) => Number(x)}
                  />
                </NInputGroup>
                <NInputGroup class="w-65%">
                  <NInputGroupLabel size="small">每张图片下载完成后休息</NInputGroupLabel>
                  <NInputNumber
                    class="w-full"
                    size="small"
                    value={store.config?.imgDownloadIntervalSec}
                    onUpdate:value={(value) => {
                      if (store.config && value !== null) {
                        store.config.imgDownloadIntervalSec = value
                      }
                    }}
                    min={0}
                    parse={(x: string) => Number(x)}
                  />
                  <NInputGroupLabel size="small">秒</NInputGroupLabel>
                </NInputGroup>
              </div>

              <NInputGroup>
                <NInputGroupLabel size="small">下载书架时，每为一本漫画创建下载任务后休息</NInputGroupLabel>
                <NInputNumber
                  class="w-full"
                  size="small"
                  min={0}
                  value={store.config?.downloadShelfIntervalMs}
                  onUpdate:value={(value) => {
                    if (store.config === undefined || value === null) {
                      return
                    }
                    store.config.downloadShelfIntervalMs = value
                  }}
                  parse={(x: string) => Number(x)}
                />
                <NInputGroupLabel size="small">毫秒</NInputGroupLabel>
              </NInputGroup>

              <NInputGroup>
                <NInputGroupLabel size="small">批量下载时，每为一本漫画创建下载任务后休息</NInputGroupLabel>
                <NInputNumber
                  class="w-full"
                  size="small"
                  min={0}
                  value={store.config?.batchDownloadIntervalMs}
                  onUpdate:value={(value) => {
                    if (store.config === undefined || value === null) {
                      return
                    }
                    store.config.batchDownloadIntervalMs = value
                  }}
                  parse={(x: string) => Number(x)}
                />
                <NInputGroupLabel size="small">毫秒</NInputGroupLabel>
              </NInputGroup>
            </div>

            <span class="font-bold mt-2">代理类型</span>
            <NRadioGroup
              size="small"
              value={store.config?.proxyMode}
              onUpdate:value={(value) => {
                if (store.config !== undefined) {
                  store.config.proxyMode = value
                }
              }}>
              <NRadioButton value="System">系统代理</NRadioButton>
              <NRadioButton value="NoProxy">直连</NRadioButton>
              <NRadioButton value="Custom">自定义</NRadioButton>
            </NRadioGroup>
            {store.config?.proxyMode === 'Custom' && (
              <NInputGroup class="mt-1">
                <NInputGroupLabel size="small">http://</NInputGroupLabel>
                <NInput
                  size="small"
                  placeholder=""
                  value={proxyHost.value}
                  onUpdate:value={(value) => (proxyHost.value = value)}
                  onBlur={() => {
                    if (store.config !== undefined) {
                      store.config.proxyHost = proxyHost.value
                    }
                  }}
                  onKeydown={(e: KeyboardEvent) => {
                    if (e.key === 'Enter' && store.config !== undefined) {
                      store.config.proxyHost = proxyHost.value
                    }
                  }}
                />
                <NInputGroupLabel size="small">:</NInputGroupLabel>
                <NInputNumber
                  size="small"
                  placeholder=""
                  value={store.config?.proxyPort}
                  onUpdate:value={(value) => {
                    if (store.config !== undefined && value !== null) {
                      store.config.proxyPort = value
                    }
                  }}
                  parse={(x: string) => parseInt(x)}
                />
              </NInputGroup>
            )}

            <span class="font-bold mt-2">其他</span>
            <NTooltip placement="top">
              {{
                trigger: () => (
                  <NCheckbox
                    class="w-fit"
                    checked={store.config?.useOriginalFilename}
                    onUpdate:checked={(value) => {
                      if (store.config) {
                        store.config.useOriginalFilename = value
                      }
                    }}>
                    使用图片原文件名
                  </NCheckbox>
                ),
                default: () => (
                  <>
                    <div class="text-red">可能导致 导出pdf 时图片顺序混乱</div>
                    <div>因为pdf的图片是根据文件名排序的</div>
                  </>
                ),
              }}
            </NTooltip>
          </div>

          <div class="flex justify-end mt-4">
            <NButton size="small" onClick={showConfigPathInFileManager}>
              打开配置目录
            </NButton>
          </div>
        </NDialog>
      </NModal>
    )
  },
})
