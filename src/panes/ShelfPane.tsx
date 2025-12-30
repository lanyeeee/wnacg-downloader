import { computed, defineComponent, ref, watch } from 'vue'
import { useStore } from '../store.ts'
import { commands } from '../bindings.ts'
import { NEmpty, NInputGroup, NInputGroupLabel, NPagination, NSelect } from 'naive-ui'
import ComicCard from '../components/ComicCard.tsx'
import DownloadShelfButton from '../components/DownloadShelfButton.tsx'

export default defineComponent({
  name: 'ShelfPane',
  setup() {
    const store = useStore()

    const shelfIdSelected = ref<number>(0)
    const currentPage = ref<number>(1)
    const comicCardContainer = ref<HTMLElement>()

    const shelfOptions = computed<{ label: string; value: number }[]>(() =>
      (store.getShelfResult?.shelves || []).map((shelf) => ({
        label: shelf.name,
        value: shelf.id,
      })),
    )

    watch(
      () => store.userProfile,
      async () => {
        if (store.userProfile === undefined) {
          store.getShelfResult = undefined
          return
        }
        await getShelf(0, 1)
      },
      { immediate: true },
    )

    async function getShelf(shelfId: number, pageNum: number) {
      shelfIdSelected.value = shelfId
      currentPage.value = pageNum
      const result = await commands.getShelf(shelfId, pageNum)
      if (result.status === 'error') {
        console.error(result.error)
        return
      }
      store.getShelfResult = result.data

      if (comicCardContainer.value !== undefined) {
        comicCardContainer.value.scrollTo({ top: 0, behavior: 'instant' })
      }
    }

    async function onPageChange(page: number) {
      if (store.getShelfResult === undefined) {
        return
      }

      currentPage.value = page
      await getShelf(shelfIdSelected.value, page)
    }

    return () => {
      if (store.userProfile === undefined) {
        return <NEmpty description="请先登录" />
      }

      if (store.getShelfResult === undefined) {
        return <NEmpty description="加载中..." />
      }

      return (
        <div class="h-full flex flex-col">
          <div class="flex items-center pt-2 px-2">
            <NInputGroup>
              <NInputGroupLabel size="small">书架</NInputGroupLabel>
              <NSelect
                class="w-40%"
                showCheckmark={false}
                value={shelfIdSelected.value}
                size="small"
                options={shelfOptions.value}
                onUpdate:value={(shelfId) => getShelf(shelfId as number, 1)}
              />
            </NInputGroup>
            <DownloadShelfButton shelfId={shelfIdSelected.value} />
          </div>

          <div class="flex flex-col overflow-auto">
            <div ref={comicCardContainer} class="flex flex-col gap-row-2 overflow-auto p-2">
              {store.getShelfResult.comics.map((comic) => (
                <ComicCard
                  key={comic.id}
                  comicId={comic.id}
                  comicTitle={comic.title}
                  comicCover={comic.cover}
                  comicDownloaded={comic.isDownloaded}
                  shelf={comic.shelf}
                  comicFavoriteTime={comic.favoriteTime}
                  getShelf={getShelf}
                />
              ))}
            </div>
          </div>
          <NPagination
            class="p-2 mt-auto"
            page={currentPage.value}
            pageCount={store.getShelfResult.totalPage}
            onUpdate:page={async (page) => await onPageChange(page)}
          />
        </div>
      )
    }
  },
})
