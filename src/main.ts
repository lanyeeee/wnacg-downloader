import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.tsx'
import 'virtual:uno.css'
import './styles/global.css'

const pinia = createPinia()
const app = createApp(App)

app.use(pinia).mount('#app')
