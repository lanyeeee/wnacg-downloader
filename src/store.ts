import { defineStore } from 'pinia'
import { Comic, Config, UserProfile } from './bindings.ts'
import { CurrentTabName, ProgressData } from './types.ts'

interface StoreState {
  config?: Config
  userProfile?: UserProfile
  pickedComic?: Comic
  currentTabName: CurrentTabName
  progresses: Map<number, ProgressData>
}

export const useStore = defineStore('store', {
  state: (): StoreState => ({
    currentTabName: 'search',
    progresses: new Map(),
  }),
})
