import { defineStore } from 'pinia'
import { Config, UserProfile } from './bindings.ts'

interface StoreState {
  config?: Config
  userProfile?: UserProfile
}

export const useStore = defineStore('store', {
  state: (): StoreState => ({}),
})
