import { computed, defineComponent, ref, watch } from 'vue'
import { useStore } from '../store.ts'
import { commands } from '../bindings.ts'
import { NEmpty, NPagination, NSelect } from 'naive-ui'
import ComicCard from '../components/ComicCard.tsx'

export default defineComponent({
  name: 'FavoritePane',
  setup() {
    const store = useStore()

    const shelfIdSelected = ref<number>(0)
    const currentPage = ref<number>(1)
    const comicCardContainer = ref<HTMLElement>()

    const shelfOptions = computed<{ label: string; value: number }[]>(() =>
      (store.getFavoriteResult?.shelves || []).map((shelf) => ({
        label: shelf.name,
        value: shelf.id,
      })),
    )

    watch(
      () => store.userProfile,
      async () => {
        if (store.userProfile === undefined) {
          store.getFavoriteResult = undefined
          return
        }
        await getFavourite(0, 1)
      },
      { immediate: true },
    )

    async function getFavourite(shelfId: number, pageNum: number) {
      shelfIdSelected.value = shelfId
      currentPage.value = pageNum
      const result = await commands.getFavorite(shelfId, pageNum)
      if (result.status === 'error') {
        console.error(result.error)
        return
      }
      store.getFavoriteResult = result.data

      if (comicCardContainer.value !== undefined) {
        comicCardContainer.value.scrollTo({ top: 0, behavior: 'instant' })
      }
    }

    async function onPageChange(page: number) {
      if (store.getFavoriteResult === undefined) {
        return
      }

      currentPage.value = page
      await getFavourite(shelfIdSelected.value, page)
    }

    return () => {
      if (store.userProfile === undefined) {
        return <NEmpty description="请先登录" />
      }

      if (store.getFavoriteResult === undefined) {
        return <NEmpty description="加载中..." />
      }

      return (
        <div class="h-full flex flex-col">
          <div class="flex items-center">
            <span class="mx-2">书架</span>
            <NSelect
              class="w-40%"
              showCheckmark={false}
              value={shelfIdSelected.value}
              size="small"
              options={shelfOptions.value}
              onUpdate:value={(shelfId) => getFavourite(shelfId as number, 1)}
            />
          </div>

          <div class="flex flex-col overflow-auto">
            <div ref={comicCardContainer} class="flex flex-col gap-row-2 overflow-auto p-2">
              {store.getFavoriteResult.comics.map((comic) => (
                <ComicCard
                  key={comic.id}
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
          <NPagination
            class="p-2 mt-auto"
            page={currentPage.value}
            pageCount={store.getFavoriteResult.totalPage}
            onUpdate:page={async (page) => await onPageChange(page)}
          />
        </div>
      )
    }
  },
})
