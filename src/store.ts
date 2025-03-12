import { defineStore } from 'pinia'

interface StoreState {
  // 占位让eslint闭嘴
  t: string
}

export const useStore = defineStore('store', {
  state: (): StoreState => ({
    t: 'hello',
  }),
})
