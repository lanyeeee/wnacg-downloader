import { computed, defineComponent, ref, watch } from 'vue'
import { Input, Button, Pagination, message } from 'ant-design-vue'
import { useStore } from '../store.ts'
import { commands } from '../bindings.ts'
import ComicCard from '../components/ComicCard.tsx'

export default defineComponent({
  name: 'SearchPane',
  setup() {
    const PAGE_SIZE = 24
    const store = useStore()

    const searchByKeywordInput = ref<string>('')
    const searchByTagInput = ref<string>('')
    const searchByComicIdInput = ref<string>('')
    const currentPage = ref<number>(1)
    const comicCardContainer = ref<HTMLElement>()

    const totalForPagination = computed(() => {
      if (store.searchResult === undefined) {
        return 1
      }
      return store.searchResult.totalPage * PAGE_SIZE
    })

    watch(
      () => store.searchResult,
      () => {
        if (comicCardContainer.value !== undefined) {
          comicCardContainer.value.scrollTo({ top: 0, behavior: 'instant' })
        }
      },
    )

    async function searchByKeyword(keyword: string, pageNum: number) {
      console.log(keyword, pageNum)
      searchByKeywordInput.value = keyword
      currentPage.value = pageNum
      const result = await commands.searchByKeyword(keyword, pageNum)
      if (result.status === 'error') {
        console.error(result.error)
        return
      }
      store.searchResult = result.data
      console.log(result.data)
    }

    async function searchByTag(tagName: string, pageNum: number) {
      console.log(tagName, pageNum)
      searchByTagInput.value = tagName
      currentPage.value = pageNum
      const result = await commands.searchByTag(tagName, pageNum)
      if (result.status === 'error') {
        console.error(result.error)
        return
      }
      store.searchResult = result.data
      store.currentTabName = 'search'
      console.log(result.data)
    }

    async function onPageChange(page: number) {
      if (store.searchResult === undefined) {
        return
      }

      if (store.searchResult.isSearchByTag) {
        await searchByTag(searchByTagInput.value.trim(), page)
      } else {
        await searchByKeyword(searchByKeywordInput.value.trim(), page)
      }
    }

    function getComicIdFromComicIdInput(): number | undefined {
      const comicIdString = searchByComicIdInput.value.trim()
      // 如果是数字，直接返回
      const comicId = parseInt(comicIdString)
      if (!isNaN(comicId)) {
        console.log(comicId)
        return comicId
      }
      // 否则需要从链接中提取
      const regex = /aid-(\d+)/
      const match = comicIdString.match(regex)
      if (match === null || match[1] === null) {
        return
      }
      console.log(match)
      return parseInt(match[1])
    }

    async function pickComic() {
      const comicId = getComicIdFromComicIdInput()
      if (comicId === undefined) {
        message.error('漫画ID格式错误，请输入漫画ID或漫画链接')
        return
      }

      const result = await commands.getComic(comicId)
      if (result.status === 'error') {
        console.error(result.error)
        return
      }

      store.pickedComic = result.data
      store.currentTabName = 'comic'
    }

    const render = () => (
      <div class="h-full flex flex-col">
        <div class="flex">
          <Input
            addonBefore="关键词"
            size="small"
            value={searchByKeywordInput.value}
            onUpdate:value={(value) => (searchByKeywordInput.value = value)}
            allowClear
            onPressEnter={() => searchByKeyword(searchByKeywordInput.value.trim(), 1)}
          />
          <Button size="small" onClick={() => searchByKeyword(searchByKeywordInput.value.trim(), 1)}>
            搜索
          </Button>
        </div>
        <div class="flex">
          <Input
            v-slots={{
              addonBefore: () => <div class="mx-1.75">标签</div>,
            }}
            size="small"
            value={searchByTagInput.value}
            onUpdate:value={(value) => (searchByTagInput.value = value)}
            allowClear
            onPressEnter={() => searchByTag(searchByTagInput.value.trim(), 1)}
          />
          <Button size="small" onClick={() => searchByTag(searchByTagInput.value.trim(), 1)}>
            搜索
          </Button>
        </div>
        <div class="flex">
          <Input
            addonBefore="漫画ID"
            placeholder="链接也行"
            size="small"
            value={searchByComicIdInput.value}
            onUpdate:value={(value) => (searchByComicIdInput.value = value)}
            allowClear
            onPressEnter={pickComic}
          />
          <Button size="small" onClick={async () => await pickComic()}>
            直达
          </Button>
        </div>
        {store.searchResult && (
          <div class="flex flex-col overflow-auto">
            <div ref={comicCardContainer} class="flex flex-col gap-row-2 overflow-auto p-2">
              {store.searchResult.comics.map((comic) => (
                <ComicCard
                  key={comic.id}
                  comicId={comic.id}
                  comicTitle={comic.title}
                  comicTitleHtml={comic.titleHtml}
                  comicCover={comic.cover}
                  comicAdditionalInfo={comic.additionalInfo}
                  comicDownloaded={comic.isDownloaded}
                />
              ))}
            </div>
          </div>
        )}
        <Pagination
          class="p-2 mt-auto"
          current={currentPage.value}
          pageSize={PAGE_SIZE}
          total={totalForPagination.value}
          showSizeChanger={false}
          simple
          onUpdate:current={async (pageNum) => await onPageChange(pageNum)}
        />
      </div>
    )

    return { render, searchByTag }
  },

  render() {
    return this.render()
  },
})
