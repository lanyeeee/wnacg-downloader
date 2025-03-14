import { defineStore } from 'pinia'
import { Comic, Config, UserProfile } from './bindings.ts'
import { CurrentTabName } from './types.ts'

interface StoreState {
  config?: Config
  userProfile?: UserProfile
  pickedComic?: Comic
  currentTabName: CurrentTabName
}

export const useStore = defineStore('store', {
  state: (): StoreState => ({
    currentTabName: 'search',
  }),
})
