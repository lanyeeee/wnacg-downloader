import { computed, defineComponent, onMounted, ref, watch } from 'vue'
import { useStore } from '../store.ts'
import { commands, events, GetFavoriteResult } from '../bindings.ts'
import { Select } from 'ant-design-vue'
import ComicCard from '../components/ComicCard.tsx'

export default defineComponent({
  name: 'FavoritePane',
  setup() {
    const store = useStore()

    const shelfIdSelected = ref<number>(0)
    const getFavoriteResult = ref<GetFavoriteResult>()
    const currentPage = ref<number>(1)
    const comicCardContainer = ref<HTMLElement>()

    const shelfOptions = computed<{ label: string; value: number }[]>(() =>
      (getFavoriteResult.value?.shelves || []).map((shelf) => ({
        label: shelf.name,
        value: shelf.id,
      })),
    )

    watch(
      () => store.userProfile,
      async () => {
        if (store.userProfile === undefined) {
          getFavoriteResult.value = undefined
          return
        }
        await getFavourite(0, 1)
      },
      { immediate: true },
    )

    onMounted(async () => {
      await events.downloadTaskEvent.listen(({ payload: downloadTaskEvent }) => {
        if (downloadTaskEvent.state !== 'Completed' || getFavoriteResult.value === undefined) {
          return
        }
        const completedResult = getFavoriteResult.value.comics.find((comic) => comic.id === downloadTaskEvent.comic.id)
        if (completedResult !== undefined) {
          completedResult.isDownloaded = true
        }
      })
    })

    async function getFavourite(shelfId: number, pageNum: number) {
      shelfIdSelected.value = shelfId
      currentPage.value = pageNum
      const result = await commands.getFavorite(shelfId, pageNum)
      if (result.status === 'error') {
        console.error(result.error)
        return
      }
      getFavoriteResult.value = result.data

      if (comicCardContainer.value !== undefined) {
        comicCardContainer.value.scrollTo({ top: 0, behavior: 'instant' })
      }
    }

    return () =>
      getFavoriteResult.value && (
        <div class="h-full flex flex-col">
          <div class="flex items-center">
            <span class="mx-2">书架</span>
            <Select
              class="w-40%"
              value={shelfIdSelected.value}
              size="small"
              options={shelfOptions.value}
              onUpdate:value={(shelfId) => getFavourite(shelfId as number, 1)}
            />
          </div>

          <div class="flex flex-col overflow-auto">
            <div ref={comicCardContainer} class="flex flex-col gap-row-2 overflow-auto p-2">
              {getFavoriteResult.value.comics.map((comic) => (
                <ComicCard
                  comicId={comic.id}
                  comicTitle={comic.title}
                  comicCover={comic.cover}
                  comicDownloaded={comic.isDownloaded}
                  shelf={comic.shelf}
                  comicFavoriteTime={comic.favoriteTime}
                  getFavorite={getFavourite}
                />
              ))}
            </div>
          </div>
        </div>
      )
  },
})
