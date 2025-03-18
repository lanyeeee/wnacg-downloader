import { defineStore } from 'pinia'
import { Comic, Config, UserProfile } from './bindings.ts'
import { CurrentTabName, ProgressData } from './types.ts'
import { ref } from 'vue'

export const useStore = defineStore('store', () => {
  const config = ref<Config>()
  const userProfile = ref<UserProfile>()
  const pickedComic = ref<Comic>()
  const currentTabName = ref<CurrentTabName>('search')
  const progresses = ref<Map<number, ProgressData>>(new Map())

  return {
    config,
    userProfile,
    pickedComic,
    currentTabName,
    progresses,
  }
})
