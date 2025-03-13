import { defineComponent } from 'vue'
import AppContent from './AppContent.tsx'
import { App as AntdApp, ConfigProvider } from 'ant-design-vue'
import zhCN from 'ant-design-vue/es/locale/zh_CN'

export default defineComponent({
  name: 'App',
  setup() {
    return () => (
      <AntdApp>
        <ConfigProvider locale={zhCN}>
          <AppContent />
        </ConfigProvider>
      </AntdApp>
    )
  },
})
