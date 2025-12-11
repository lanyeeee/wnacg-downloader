import { defineComponent, ref, watch } from 'vue'
import { NInput, NButton, NPagination, useMessage, NInputGroup, NInputGroupLabel } from 'naive-ui'
import { useStore } from '../store.ts'
import { commands } from '../bindings.ts'
import ComicCard from '../components/ComicCard.tsx'

export default defineComponent({
  name: 'SearchPane',
  setup() {
    const store = useStore()

    const message = useMessage()

    const searchByKeywordInput = ref<string>('')
    const searchByTagInput = ref<string>('')
    const searchByComicIdInput = ref<string>('')
    const currentPage = ref<number>(1)
    const comicCardContainer = ref<HTMLElement>()

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
        <NInputGroup>
          <NInputGroupLabel size="small">关键词</NInputGroupLabel>
          <NInput
            size="small"
            placeholder=""
            value={searchByKeywordInput.value}
            onUpdate:value={(value) => (searchByKeywordInput.value = value)}
            clearable
            onKeydown={(e: KeyboardEvent) => {
              if (e.key === 'Enter') {
                searchByKeyword(searchByKeywordInput.value.trim(), 1)
              }
            }}
          />
          <NButton size="small" onClick={() => searchByKeyword(searchByKeywordInput.value.trim(), 1)}>
            搜索
          </NButton>
        </NInputGroup>
        <NInputGroup>
          <NInputGroupLabel size="small">&ensp;标签&ensp;</NInputGroupLabel>
          <NInput
            size="small"
            placeholder=""
            value={searchByTagInput.value}
            onUpdate:value={(value) => (searchByTagInput.value = value)}
            clearable
            onKeydown={(e: KeyboardEvent) => {
              if (e.key === 'Enter') {
                searchByTag(searchByTagInput.value.trim(), 1)
              }
            }}
          />
          <NButton size="small" onClick={() => searchByTag(searchByTagInput.value.trim(), 1)}>
            搜索
          </NButton>
        </NInputGroup>
        <NInputGroup>
          <NInputGroupLabel size="small">漫画ID</NInputGroupLabel>
          <NInput
            placeholder="链接也行"
            size="small"
            value={searchByComicIdInput.value}
            onUpdate:value={(value) => (searchByComicIdInput.value = value)}
            clearable
            onKeydown={(e: KeyboardEvent) => {
              if (e.key === 'Enter') {
                pickComic()
              }
            }}
          />
          <NButton size="small" onClick={() => pickComic()}>
            直达
          </NButton>
        </NInputGroup>

        {store.searchResult && (
          <>
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
            <NPagination
              class="p-2 mt-auto"
              page={currentPage.value}
              pageCount={store.searchResult.totalPage}
              onUpdate:page={async (page) => await onPageChange(page)}
            />
          </>
        )}
      </div>
    )

    return { render, searchByTag }
  },

  render() {
    return this.render()
  },
})
