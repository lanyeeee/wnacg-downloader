import { computed, defineComponent, nextTick, ref, watch } from 'vue'
import { useStore } from '../../../store.ts'
import { SelectionArea, SelectionEvent } from '@viselect/vue'
import { ProgressData } from '../../../types.ts'
import { Comic, commands, DownloadTaskState } from '../../../bindings.ts'
import {
  PhPause,
  PhChecks,
  PhTrash,
  PhCaretRight,
  PhCloudArrowDown,
  PhClock,
  PhWarningCircle,
  PhPlusCircle,
} from '@phosphor-icons/vue'
import { NDropdown, NProgress, ProgressProps, DropdownOption, NIcon, NButton } from 'naive-ui'
import styles from './UncompletedProgresses.module.css'
import BatchDownloadDialog from '../../../dialogs/BatchDownloadDialog.tsx'

export default defineComponent({
  name: 'UncompletedProgress',
  setup: function () {
    const store = useStore()

    const batchDownloadDialogShowing = ref<boolean>(false)

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

    const dropdownX = ref<number>(0)
    const dropdownY = ref<number>(0)
    const dropdownShowing = ref<boolean>(false)
    const dropdownOptions: DropdownOption[] = [
      {
        label: '全选',
        key: 'check all',
        icon: () => (
          <NIcon size="20">
            <PhChecks />
          </NIcon>
        ),
        props: {
          onClick: () => {
            if (selectionAreaRef.value === undefined) {
              return
            }
            const selection = selectionAreaRef.value.selection
            if (selection === undefined) {
              return
            }
            selection.select(selectableRefs.value)
            dropdownShowing.value = false
          },
        },
      },
      {
        label: '继续',
        key: 'resume',
        icon: () => (
          <NIcon size="20">
            <PhCaretRight />
          </NIcon>
        ),
        props: {
          onClick: () => {
            selectedIds.value.forEach(async (comicId) => {
              const result = await commands.resumeDownloadTask(comicId)
              if (result.status === 'error') {
                console.error(result.error)
              }
            })
            dropdownShowing.value = false
          },
        },
      },
      {
        label: '暂停',
        key: 'pause',
        icon: () => (
          <NIcon size="20">
            <PhPause />
          </NIcon>
        ),
        props: {
          onClick: () => {
            selectedIds.value.forEach(async (comicId) => {
              const result = await commands.pauseDownloadTask(comicId)
              if (result.status === 'error') {
                console.error(result.error)
              }
            })
            dropdownShowing.value = false
          },
        },
      },
      {
        label: '取消',
        key: 'cancel',
        icon: () => (
          <NIcon size="20">
            <PhTrash />
          </NIcon>
        ),
        props: {
          onClick: () => {
            selectedIds.value.forEach(async (comicId) => {
              const result = await commands.cancelDownloadTask(comicId)
              if (result.status === 'error') {
                console.error(result.error)
              }
            })
            dropdownShowing.value = false
          },
        },
      },
    ]

    async function showDropdown(e: MouseEvent) {
      dropdownShowing.value = false
      await nextTick()
      dropdownShowing.value = true
      dropdownX.value = e.clientX
      dropdownY.value = e.clientY
    }

    return () => (
      <div class="h-full flex flex-col gap-2 box-border">
        <div class="flex items-center select-none pt-0.5 px-2">
          <div class="animate-pulse text-sm text-blue-6 flex flex-col">
            <div>左键拖动进行框选，右键打开菜单</div>
            <div>双击暂停/继续</div>
          </div>
          <NButton
            class="ml-auto"
            size="small"
            type="primary"
            onClick={() => (batchDownloadDialogShowing.value = true)}>
            {{
              icon: () => (
                <NIcon size={24}>
                  <PhPlusCircle />
                </NIcon>
              ),
              default: () => <div>批量下载</div>,
            }}
          </NButton>
        </div>

        <SelectionArea
          ref={selectionAreaRef}
          class={`${styles.selectionContainer} select-none overflow-auto h-full flex flex-col`}
          options={{ selectables: '.selectable', features: { deselectOnBlur: true } }}
          // 如果直接用 onContextmenu={showDropdown}，运行没问题，但是ts会报错
          // 在vue里用jsx总有类似的狗屎问题 https://github.com/vuejs/babel-plugin-jsx/issues/555
          {...{
            onContextmenu: showDropdown,
          }}
          onMove={updateSelectedIds}
          onStart={unselectAll}>
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
          <NDropdown
            placement="bottom-start"
            trigger="manual"
            x={dropdownX.value}
            y={dropdownY.value}
            options={dropdownOptions}
            show={dropdownShowing.value}
            on-clickoutside={() => (dropdownShowing.value = false)}
          />
        </SelectionArea>

        <BatchDownloadDialog
          showing={batchDownloadDialogShowing.value}
          onUpdate:showing={(value) => (batchDownloadDialogShowing.value = value)}
        />
      </div>
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
  const colorClass = stateToColorClass(state)

  return (
    <div class="flex flex-col">
      <div class="text-ellipsis whitespace-nowrap overflow-hidden" title={comic.title}>
        {comic.title}
      </div>
      <div class="flex">
        <NIcon class={[colorClass, 'mr-2']} size={20}>
          {state === 'Downloading' && <PhCloudArrowDown />}
          {state === 'Pending' && <PhClock />}
          {state === 'Paused' && <PhPause />}
          {state === 'Failed' && <PhWarningCircle />}
        </NIcon>
        {!started && <div class="ml-auto">{indicator}</div>}
        {started && (
          <NProgress
            class={colorClass}
            status={stateToStatus(state)}
            percentage={percentage}
            processing={state === 'Downloading'}>
            {indicator}
          </NProgress>
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
  if (state === 'Completed') {
    return 'success'
  } else if (state === 'Paused') {
    return 'warning'
  } else if (state === 'Failed') {
    return 'error'
  } else {
    return 'default'
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
