import { computed, defineComponent, ref, watch } from 'vue'
import { useStore } from '../store.ts'
import { SelectionArea, SelectionEvent } from '@viselect/vue'
import { ProgressData } from '../types.ts'
import { Comic, commands, DownloadTaskState } from '../bindings.ts'
import {
  CheckOutlined,
  ClockCircleOutlined,
  DeleteOutlined,
  ExclamationCircleOutlined,
  LoadingOutlined,
  PauseOutlined,
  RightOutlined,
} from '@ant-design/icons-vue'
import { Dropdown, Menu, MenuProps, Progress, ProgressProps } from 'ant-design-vue'
import styles from '../styles/UncompletedProgresses.module.css'

export default defineComponent({
  name: 'UncompletedProgress',
  setup: function () {
    const store = useStore()

    const selectedIds = ref<Set<number>>(new Set())
    const selectionAreaRef = ref<InstanceType<typeof SelectionArea>>()
    const selectableRefs = ref<HTMLDivElement[]>([])

    const uncompletedProgresses = computed<[number, ProgressData][]>(() =>
      Array.from(store.progresses.entries())
        .filter(([, { state }]) => state !== 'Completed' && state !== 'Cancelled')
        .sort((a, b) => b[1].totalImgCount - a[1].totalImgCount),
    )

    watch(uncompletedProgresses, () => {
      const uncompletedIds = new Set(uncompletedProgresses.value.map(([chapterId]) => chapterId))
      // 只留下未完成的漫画
      selectedIds.value = new Set([...selectedIds.value].filter((comicId) => uncompletedIds.has(comicId)))
    })

    function updateSelectedIds({
      store: {
        changed: { added, removed },
      },
    }: SelectionEvent) {
      extractIds(added).forEach((comicId) => selectedIds.value.add(comicId))
      extractIds(removed).forEach((comicId) => selectedIds.value.delete(comicId))
    }

    function unselectAll({ event, selection }: SelectionEvent) {
      if (!event?.ctrlKey && !event?.metaKey) {
        selection.clearSelection()
        selectedIds.value.clear()
      }
    }

    async function handleProgressDoubleClick(state: DownloadTaskState, comicId: number) {
      if (state === 'Downloading' || state === 'Pending') {
        const result = await commands.pauseDownloadTask(comicId)
        if (result.status === 'error') {
          console.error(result.error)
        }
      } else {
        const result = await commands.resumeDownloadTask(comicId)
        if (result.status === 'error') {
          console.error(result.error)
        }
      }
    }

    function handleProgressContextMenu(comicId: number) {
      if (selectedIds.value.has(comicId)) {
        return
      }
      selectedIds.value.clear()
      selectedIds.value.add(comicId)
    }

    const dropdownOptions: MenuProps['items'] = [
      {
        label: '全选',
        key: 'check all',
        icon: <CheckOutlined />,
        onClick: () => {
          if (selectionAreaRef.value === undefined) {
            return
          }
          const selection = selectionAreaRef.value.selection
          if (selection === undefined) {
            return
          }
          selection.select(selectableRefs.value)
        },
      },
      {
        label: '继续',
        key: 'resume',
        icon: <RightOutlined />,
        onClick: () => {
          selectedIds.value.forEach(async (comicId) => {
            const result = await commands.resumeDownloadTask(comicId)
            if (result.status === 'error') {
              console.error(result.error)
            }
          })
        },
      },
      {
        label: '暂停',
        key: 'pause',
        icon: <PauseOutlined />,
        onClick: () => {
          selectedIds.value.forEach(async (comicId) => {
            const result = await commands.pauseDownloadTask(comicId)
            if (result.status === 'error') {
              console.error(result.error)
            }
          })
        },
      },
      {
        label: '取消',
        key: 'cancel',
        icon: <DeleteOutlined />,
        onClick: () => {
          selectedIds.value.forEach(async (comicId) => {
            const result = await commands.cancelDownloadTask(comicId)
            if (result.status === 'error') {
              console.error(result.error)
            }
          })
        },
      },
    ]

    return () => (
      <SelectionArea
        ref={selectionAreaRef}
        class={`${styles.selectionContainer} select-none overflow-auto h-full flex flex-col`}
        options={{ selectables: '.selectable', features: { deselectOnBlur: true } }}
        onMove={updateSelectedIds}
        onStart={unselectAll}>
        <div class="h-6 flex-shrink-0 items-center ml-auto">左键拖动进行框选，右键打开菜单，双击暂停/继续</div>
        <Dropdown
          class="select-none"
          trigger={['contextmenu']}
          v-slots={{
            overlay: () => <Menu items={dropdownOptions}></Menu>,
          }}>
          <div class="h-full select-none">
            {uncompletedProgresses.value.map(([comicId, { state, comic, percentage, indicator }]) => (
              <div
                key={comicId}
                ref={(el) => {
                  selectableRefs.value[comicId] = el as HTMLDivElement
                }}
                data-key={comicId}
                class={[
                  'selectable p-3 mb-2 rounded-lg',
                  selectedIds.value.has(comicId) ? 'selected shadow-md' : 'hover:bg-gray-1',
                ]}
                onDblclick={() => handleProgressDoubleClick(state, comicId)}
                onContextmenu={() => handleProgressContextMenu(comicId)}>
                <DownloadProgress percentage={percentage} state={state} comic={comic} indicator={indicator} />
              </div>
            ))}
          </div>
        </Dropdown>
      </SelectionArea>
    )
  },
})

function DownloadProgress({
  percentage,
  state,
  comic,
  indicator,
}: {
  percentage: number
  state: DownloadTaskState
  comic: Comic
  indicator: string
}) {
  const started = !isNaN(percentage)
  const color = stateToColorHex(state)
  const colorClass = stateToColorClass(state)

  return (
    <div class="flex flex-col">
      <div class="text-ellipsis whitespace-nowrap overflow-hidden" title={comic.title}>
        {comic.title}
      </div>
      <div class="flex">
        {state === 'Downloading' && <LoadingOutlined class={`text-lg ${colorClass}`} spin />}
        {state === 'Pending' && <ClockCircleOutlined class={`text-lg ${colorClass}`} />}
        {state === 'Paused' && <PauseOutlined class={`text-lg ${colorClass}`} />}
        {state === 'Failed' && <ExclamationCircleOutlined class={`text-lg ${colorClass}`} />}
        {!started && <div class="ml-auto">{indicator}</div>}
        {started && (
          <>
            <Progress
              class="ml-2 mt-1"
              strokeColor={color}
              status={stateToStatus(state)}
              percent={percentage}
              showInfo={false}
            />
            <div class={`flex items-center whitespace-nowrap ${colorClass}`}>{indicator}</div>
          </>
        )}
      </div>
    </div>
  )
}

function extractIds(elements: Element[]): number[] {
  return elements
    .map((element) => element.getAttribute('data-key'))
    .filter(Boolean)
    .map(Number)
}

function stateToStatus(state: DownloadTaskState): ProgressProps['status'] {
  if (state === 'Downloading') {
    return 'active'
  } else if (state === 'Completed') {
    return 'success'
  } else if (state === 'Failed') {
    return 'exception'
  } else {
    return 'normal'
  }
}

function stateToColorClass(state: DownloadTaskState) {
  if (state === 'Downloading') {
    return 'text-blue-500'
  } else if (state === 'Pending') {
    return 'text-gray-500'
  } else if (state === 'Paused') {
    return 'text-yellow-500'
  } else if (state === 'Failed') {
    return 'text-red-500'
  } else if (state === 'Completed') {
    return 'text-green-500'
  } else if (state === 'Cancelled') {
    return 'text-stone-500'
  }

  return ''
}

function stateToColorHex(state: DownloadTaskState) {
  if (state === 'Downloading') {
    return '#2B7FFF'
  } else if (state === 'Pending') {
    return '#6A7282'
  } else if (state === 'Paused') {
    return '#F0B100'
  } else if (state === 'Failed') {
    return '#FB2C36'
  } else if (state === 'Completed') {
    return '#00C950'
  } else if (state === 'Cancelled') {
    return '#79716B'
  }

  return ''
}
