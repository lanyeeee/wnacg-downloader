import { defineStore } from 'pinia'
import { Comic, commands, Config, GetFavoriteResult, SearchResult, UserProfile } from './bindings.ts'
import { CurrentTabName, ProgressData } from './types.ts'
import { ref } from 'vue'

export const useStore = defineStore('store', () => {
  const config = ref<Config>()
  const userProfile = ref<UserProfile>()
  const pickedComic = ref<Comic>()
  const currentTabName = ref<CurrentTabName>('search')
  const progresses = ref<Map<number, ProgressData>>(new Map())
  const getFavoriteResult = ref<GetFavoriteResult>()
  const searchResult = ref<SearchResult>()
  const covers = ref<Map<number, string>>(new Map())

  async function loadCover(id: number, url: string) {
    const result = await commands.getCoverData(url)
    if (result.status === 'error') {
      console.error(result.error)
      return
    }
    const coverData: number[] = result.data
    const coverBlob = new Blob([new Uint8Array(coverData)])
    const cover = URL.createObjectURL(coverBlob)
    covers.value.set(id, cover)
  }

  return {
    config,
    userProfile,
    pickedComic,
    currentTabName,
    progresses,
    getFavoriteResult,
    searchResult,
    covers,
    loadCover,
  }
})
