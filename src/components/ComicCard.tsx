import { computed, defineComponent, onMounted, PropType } from 'vue'
import { useStore } from '../store.ts'
import { commands, Shelf } from '../bindings.ts'
import { path } from '@tauri-apps/api'
import { Button, Card } from 'ant-design-vue'
import DownloadButton from './DownloadButton.tsx'
import styles from '../styles/ComicCard.module.css'

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
    shelf: {
      type: Object as PropType<Shelf>,
      required: false,
    },
    comicFavoriteTime: {
      type: String,
      required: false,
    },
    getFavorite: {
      type: Function as PropType<(shelfId: number, pageNum: number) => Promise<void>>,
      required: false,
    },
  },
  setup(props) {
    const store = useStore()

    const cover = computed<string | undefined>(() => store.covers.get(props.comicId))

    onMounted(async () => {
      if (cover.value !== undefined) {
        return
      }

      await store.loadCover(props.comicId, props.comicCover)
    })

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
      <Card hoverable={true} class={`${styles.comicCard} cursor-auto rounded-none`} bodyStyle={{ padding: '0.25rem' }}>
        <div class="flex h-full">
          <img
            class="w-24 object-contain mr-4 cursor-pointer transition-transform duration-200 hover:scale-106"
            src={cover.value}
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
            {props.comicFavoriteTime && <span>收藏时间：{props.comicFavoriteTime}</span>}
            {props.shelf && props.getFavorite && (
              <div>
                <span>所属书架：</span>
                {props.shelf.name !== '' && (
                  <Button
                    size="small"
                    onClick={async () => {
                      if (props.shelf !== undefined && props.getFavorite !== undefined) {
                        await props.getFavorite(props.shelf.id, 1)
                      }
                    }}>
                    {props.shelf.name}
                  </Button>
                )}
              </div>
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
