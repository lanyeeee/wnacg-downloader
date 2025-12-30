import { computed, defineComponent, onMounted, PropType } from 'vue'
import { useStore } from '../../../store.ts'
import { Comic, commands } from '../../../bindings.ts'
import { NCard } from 'naive-ui'
import { path } from '@tauri-apps/api'
import { PhFilePdf, PhFileZip, PhFolderOpen } from '@phosphor-icons/vue'
import IconButton from '../../../components/IconButton.tsx'

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

    const cover = computed<string | undefined>(() => store.covers.get(props.comic.id))

    onMounted(async () => {
      if (cover.value !== undefined) {
        return
      }

      await store.loadCover(props.comic.id, props.comic.cover)
    })

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
      <NCard hoverable content-style="padding: 0.25rem;">
        <div class="flex h-full">
          <img
            class="w-24 object-contain mr-4 cursor-pointer transition-transform duration-200 hover:scale-106"
            src={cover.value}
            alt=""
            onClick={pickComic}
          />
          <div class="flex flex-col w-full">
            <span
              class="font-bold text-lg line-clamp-3 cursor-pointer transition-colors duration-200 hover:text-blue-5"
              v-html={props.comic.title}
              onClick={pickComic}
            />
            <span>分类：{props.comic.category}</span>
            <span>页数：{props.comic.imageCount}P</span>
            <div class="flex mt-auto gap-col-2">
              <IconButton title="打开下载目录" onClick={showComicDirInFileManager}>
                <PhFolderOpen size={24} />
              </IconButton>
              <IconButton class="ml-auto" title="导出cbz" onClick={exportCbz}>
                <PhFileZip size={24} />
              </IconButton>
              <IconButton title="导出pdf" onClick={exportPdf}>
                <PhFilePdf size={24} />
              </IconButton>
            </div>
          </div>
        </div>
      </NCard>
    )
  },
})
