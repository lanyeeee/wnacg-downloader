import { defineComponent, PropType } from 'vue'

export default defineComponent({
  name: 'IconButton',
  props: {
    title: {
      type: String,
      default: '',
    },
    onClick: {
      type: Function as PropType<() => void>,
    },
  },
  setup(props, { slots }) {
    return () => (
      <div
        class="cursor-pointer p-1 rounded-lg flex items-center justify-between text-gray-6 hover:bg-[#1677FF] hover:text-white active:bg-[#0958D9] active:text-white"
        onClick={props.onClick}
        title={props.title}>
        {slots.default && slots.default()}
      </div>
    )
  },
})
