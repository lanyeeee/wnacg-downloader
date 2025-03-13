import { defineStore } from 'pinia'
import { Config } from './bindings.ts'

interface StoreState {
  config?: Config
}

export const useStore = defineStore('store', {
  state: (): StoreState => ({}),
})
