import { computed, defineComponent } from 'vue'
import { useStore } from '../store.ts'
import { ProgressData } from '../types.ts'
import DownloadedComicCard from './DownloadedComicCard.tsx'

export default defineComponent({
  name: 'CompletedProgress',
  setup() {
    const store = useStore()

    const completedProgresses = computed<[number, ProgressData][]>(() =>
      Array.from(store.progresses.entries())
        .filter(([, { state }]) => state === 'Completed')
        .sort((a, b) => {
          return b[1].totalImgCount - a[1].totalImgCount
        }),
    )

    return () => (
      <div class="h-full">
        <div class="flex flex-col gap-row-2 overflow-auto p-2">
          {completedProgresses.value.map(([id, { comic }]) => (
            <DownloadedComicCard key={id} comic={comic} />
          ))}
        </div>
      </div>
    )
  },
})
