import { defineComponent } from 'vue'
import { useStore } from '../store.ts'
import { commands } from '../bindings.ts'
import { path } from '@tauri-apps/api'
import { Button, Card } from 'ant-design-vue'
import DownloadButton from './DownloadButton.tsx'

export default defineComponent({
  name: 'ComicCard',
  props: {
    comicId: {
      type: Number,
      required: true,
    },
    comicTitle: {
      type: String,
      required: true,
    },
    comicTitleHtml: {
      type: String,
      required: false,
    },
    comicCover: {
      type: String,
      required: true,
    },
    comicAdditionalInfo: {
      type: String,
      required: false,
    },
    comicDownloaded: {
      type: Boolean,
      required: true,
    },
  },
  setup(props) {
    const store = useStore()

    // 获取漫画信息，将漫画信息存入pickedComic，并切换到漫画详情
    async function pickComic() {
      const result = await commands.getComic(props.comicId)
      if (result.status === 'error') {
        console.error(result.error)
        return
      }

      store.pickedComic = result.data
      store.currentTabName = 'comic'
    }

    async function showComicDirInFileManager() {
      if (store.config === undefined) {
        return
      }

      const comicDir = await path.join(store.config.downloadDir, props.comicTitle)

      const result = await commands.showPathInFileManager(comicDir)
      if (result.status === 'error') {
        console.error(result.error)
      }
    }

    return () => (
      <Card hoverable={true} class="cursor-auto rounded-none" bodyStyle={{ padding: '0.25rem' }}>
        <div class="flex h-full">
          <img
            class="w-24 object-contain mr-4 cursor-pointer transition-transform duration-200 hover:scale-106"
            src={props.comicCover}
            alt=""
            onClick={pickComic}
          />
          <div class="flex flex-col w-full">
            <span
              class="font-bold text-xl line-clamp-3 cursor-pointer transition-colors duration-200 hover:text-blue-5"
              v-html={props.comicTitleHtml ?? props.comicTitle}
              onClick={pickComic}
            />
            {props.comicAdditionalInfo && (
              <span class="text-gray whitespace-pre-wrap">{props.comicAdditionalInfo}</span>
            )}
            <div class="flex mt-auto">
              {props.comicDownloaded && (
                <Button size="small" onClick={showComicDirInFileManager}>
                  打开目录
                </Button>
              )}
              <DownloadButton
                class="ml-auto"
                size="small"
                type="primary"
                comicId={props.comicId}
                comicDownloaded={props.comicDownloaded}
              />
            </div>
          </div>
        </div>
      </Card>
    )
  },
})
