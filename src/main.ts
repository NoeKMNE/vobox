import { createApp } from 'vue'
import { router } from './router'
import { initTheme } from './composables/useTheme'
import { initAccent } from './composables/useAccent'
import { initUpdater } from './composables/useUpdater'
import './style.css'
import App from './App.vue'

initTheme()
initAccent()

const app = createApp(App)
app.use(router).mount('#app')

void initUpdater()
