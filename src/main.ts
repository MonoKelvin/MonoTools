import { createApp } from 'vue'
import { createPinia } from 'pinia'
import PrimeVue from 'primevue/config'
import Aura from '@primeuix/themes/aura'
import App from './App.vue'
import { MonoToolsPreset } from './styles/themes/monotools'
import './styles/variables.css'
import './styles/primevue-override.css'
import './styles/animations.css'
import './styles/utilities.css'

const app = createApp(App)
const pinia = createPinia()

app.use(pinia)
app.use(PrimeVue, {
  theme: {
    preset: MonoToolsPreset,
    options: {
      darkModeSelector: '.mt-dark',
      cssLayer: {
        name: 'primevue',
        order: 'tailwind-base, primevue, tailwind-utilities'
      }
    }
  }
})

app.mount('#app')
