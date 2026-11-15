import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { router } from './router'
import App from './App.vue'
import './assets/styles/main.css'
import '@phosphor-icons/web/regular'
import '@phosphor-icons/web/bold'
import '@phosphor-icons/web/fill'
import '@phosphor-icons/web/duotone'
import '@phosphor-icons/web/light'
import '@phosphor-icons/web/thin'
import { setupExternalLinksInterceptor } from './utils/externalLinks'

// Initialiser l'interception globale de tous les liens externes
setupExternalLinksInterceptor()

const app = createApp(App)
const pinia = createPinia()

app.use(pinia)
app.use(router)

app.mount('#app')
