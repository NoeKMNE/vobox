import { createApp } from 'vue'
import { router } from './router'
import { initTheme } from './composables/useTheme'
import { initAccent } from './composables/useAccent'
import { initCursor } from './composables/useCursor'
import './style.css'
import App from './App.vue'

initTheme()
initAccent()
initCursor()

createApp(App).use(router).mount('#app')
