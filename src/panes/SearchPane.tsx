import { defineComponent, ref, watch } from 'vue'
import { NButton, NPagination, useMessage, NInputGroup, NIcon } from 'naive-ui'
import { useStore } from '../store.ts'
import { commands, type CommandError } from '../bindings.ts'
import ComicCard from '../components/ComicCard.tsx'
import { PhArrowRight, PhMagnifyingGlass } from '@phosphor-icons/vue'
import FloatLabelInput from '../components/FloatLabelInput.tsx'
import { extractComicId } from '../utils.ts'

export default defineComponent({
  name: 'SearchPane',
  setup() {
    const store = useStore()

    const message = useMessage()

    const searchByKeywordInput = ref<string>('')
    const searchingByKeyword = ref<boolean>(false)
    const searchByTagInput = ref<string>('')
    const searchingByTag = ref<boolean>(false)
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

    function showSearchError(error: CommandError) {
      console.error(error)
      let reason = error.err_message
      if (/timed out|timeout/i.test(reason)) {
        reason = '请求超时，请检查网络或代理后重试（连接超时 10 秒，请求总超时 30 秒）'
      } else if (/error sending request|connect|dns/i.test(reason)) {
        reason = '无法连接网站，请检查网络、代理和配置中的网站域名'
      } else if (/SearchResult|html|总结果数/i.test(reason)) {
        reason = '无法解析网站返回的搜索结果，请在日志中查看详细原因'
      }
      message.error(`${error.err_title}：${reason.slice(0, 300)}`, {
        duration: 8000,
        closable: true,
      })
    }

    async function searchByKeyword(keyword: string, pageNum: number) {
      if (searchingByKeyword.value) return
      searchByKeywordInput.value = keyword
      currentPage.value = pageNum

      searchingByKeyword.value = true

      try {
        const result = await commands.searchByKeyword(keyword, pageNum)
        if (result.status === 'error') {
          showSearchError(result.error)
          return
        }
        store.searchResult = result.data
      } catch {
        message.error('关键词搜索失败：应用通信异常，请重试')
      } finally {
        searchingByKeyword.value = false
      }
    }

    async function searchByTag(tagName: string, pageNum: number) {
      if (searchingByTag.value) return
      searchByTagInput.value = tagName
      currentPage.value = pageNum

      searchingByTag.value = true

      try {
        const result = await commands.searchByTag(tagName, pageNum)
        if (result.status === 'error') {
          showSearchError(result.error)
          return
        }
        store.searchResult = result.data
        store.currentTabName = 'search'
      } catch {
        message.error('标签搜索失败：应用通信异常，请重试')
      } finally {
        searchingByTag.value = false
      }
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

    async function pickComic() {
      const comicId = extractComicId(searchByComicIdInput.value)
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
      <div class="h-full flex flex-col gap-2">
        <NInputGroup class="box-border px-2 pt-2">
          <FloatLabelInput
            size="small"
            label="关键词"
            value={searchByKeywordInput.value}
            onUpdate:value={(value) => (searchByKeywordInput.value = value)}
            clearable
            {...{
              onKeydown: async (e: KeyboardEvent) => {
                if (e.key === 'Enter') {
                  await searchByKeyword(searchByKeywordInput.value.trim(), 1)
                }
              },
            }}
          />
          <NButton
            loading={searchingByKeyword.value}
            type="primary"
            size="small"
            class="w-15%"
            onClick={() => searchByKeyword(searchByKeywordInput.value.trim(), 1)}>
            {{
              icon: () => (
                <NIcon size={22}>
                  <PhMagnifyingGlass />
                </NIcon>
              ),
            }}
          </NButton>
        </NInputGroup>
        <NInputGroup class="box-border px-2">
          <FloatLabelInput
            size="small"
            label="标签"
            value={searchByTagInput.value}
            onUpdate:value={(value) => (searchByTagInput.value = value)}
            clearable
            {...{
              onKeydown: async (e: KeyboardEvent) => {
                if (e.key === 'Enter') {
                  await searchByTag(searchByTagInput.value.trim(), 1)
                }
              },
            }}
          />
          <NButton
            loading={searchingByTag.value}
            type="primary"
            size="small"
            class="w-15%"
            onClick={() => searchByTag(searchByTagInput.value.trim(), 1)}>
            {{
              icon: () => (
                <NIcon size={22}>
                  <PhMagnifyingGlass />
                </NIcon>
              ),
            }}
          </NButton>
        </NInputGroup>
        <NInputGroup class="box-border px-2">
          <FloatLabelInput
            size="small"
            label="漫画ID (链接也行)"
            value={searchByComicIdInput.value}
            onUpdate:value={(value) => (searchByComicIdInput.value = value)}
            clearable
            {...{
              onKeydown: async (e: KeyboardEvent) => {
                if (e.key === 'Enter') {
                  await pickComic()
                }
              },
            }}
          />
          <NButton type="primary" size="small" class="w-15%" onClick={() => pickComic()}>
            {{
              icon: () => (
                <NIcon size={22}>
                  <PhArrowRight />
                </NIcon>
              ),
            }}
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
              onUpdate:page={(page) => onPageChange(page)}
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
