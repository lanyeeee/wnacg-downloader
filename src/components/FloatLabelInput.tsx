import { NInput, NEl } from 'naive-ui'
import type { InputInst, InputProps } from 'naive-ui'
import { computed, ref, defineComponent, PropType } from 'vue'
import styles from './FloatLabelInput.module.css'

export default defineComponent({
  name: 'FloatLabelInput',
  props: {
    label: {
      type: String,
      required: true,
    },
    size: {
      type: String as () => InputProps['size'],
      default: 'medium',
    },
    type: {
      type: String as () => InputProps['type'],
      default: 'text',
    },
    clearable: {
      type: Boolean,
      default: false,
    },
    value: {
      type: String as PropType<InputProps['value']>,
      required: true,
    },
  },
  emits: { 'update:value': (_value: string) => true },
  setup(props, { emit }) {
    const focused = ref(false)
    const NInputRef = ref<InputInst>()

    const floating = computed(() => props.value !== '' || focused.value)

    const translateY = computed(() => {
      if (props.size === 'tiny') {
        return 'translate-y-[-90%]'
      } else if (props.size === 'small') {
        return 'translate-y-[-120%]'
      } else if (props.size === 'medium') {
        return 'translate-y-[-140%]'
      } else if (props.size === 'large') {
        return 'translate-y-[-160%]'
      }
      return ''
    })

    const render = () => (
      <NInput
        class={styles.floatLabelInput}
        ref={NInputRef}
        size={props.size}
        type={props.type}
        clearable={props.clearable}
        placeholder=""
        value={props.value}
        onUpdateValue={(value) => emit('update:value', value)}
        onFocus={() => (focused.value = true)}
        onBlur={() => (focused.value = false)}>
        {{
          prefix: () => (
            <NEl
              tag="span"
              class={[
                `${styles.floatLabel} bg-white transition-all duration-200 ease-in-out`,
                floating.value ? `text-0.75rem px-0.5 ${translateY.value}` : '',
              ]}>
              {props.label}
            </NEl>
          ),
        }}
      </NInput>
    )

    return { NInputRef, render }
  },
  render() {
    return this.render()
  },
})
