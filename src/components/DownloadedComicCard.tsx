import { defineComponent, PropType } from 'vue'
import { useStore } from '../store.ts'
import { Comic, commands } from '../bindings.ts'
import { Button, Card } from 'ant-design-vue'
import { path } from '@tauri-apps/api'

export default defineComponent({
  name: 'DownloadedComicCard',
  props: {
    comic: {
      type: Object as PropType<Comic>,
      required: true,
    },
  },
  setup(props) {
    const store = useStore()

    async function pickComic() {
      store.pickedComic = props.comic
      store.currentTabName = 'comic'
    }

    async function exportCbz() {
      const result = await commands.exportCbz(props.comic)
      if (result.status === 'error') {
        console.error(result.error)
        return
      }
    }

    async function exportPdf() {
      const result = await commands.exportPdf(props.comic)
      if (result.status === 'error') {
        console.error(result.error)
        return
      }
    }

    async function showComicDirInFileManager() {
      if (store.config === undefined) {
        return
      }

      const comicDir = await path.join(store.config.downloadDir, props.comic.title)

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
            src={props.comic.cover}
            alt=""
            onClick={pickComic}
          />
          <div class="flex flex-col w-full">
            <span
              class="font-bold text-xl line-clamp-3 cursor-pointer transition-colors duration-200 hover:text-blue-5"
              v-html={props.comic.title}
              onClick={pickComic}
            />
            <span>分类：{props.comic.category}</span>
            <span>页数：{props.comic.imageCount}P</span>
            <div class="flex mt-auto gap-col-2">
              <Button size="small" onClick={showComicDirInFileManager}>
                打开目录
              </Button>
              <Button class="ml-auto" size="small" onClick={exportPdf}>
                导出pdf
              </Button>
              <Button size="small" onClick={exportCbz}>
                导出cbz
              </Button>
            </div>
          </div>
        </div>
      </Card>
    )
  },
})
