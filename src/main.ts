import { createApp } from 'vue'
import App from './App.vue'

import './assets/styles/main.scss'

import PrimeVue from 'primevue/config'
import Aura from '@primeuix/themes/aura'
import Tooltip from 'primevue/tooltip'

import { router } from './router'
import { pinia } from './stores'

const app = createApp(App)

app.use(pinia)
app.use(router)
app.use(PrimeVue, {
  theme: {
    preset: Aura,
    options: {
      darkModeSelector: '.theme-dark, :root',
      cssLayer: false,
    },
  },
  ripple: true,
})
app.directive('tooltip', Tooltip)

app.mount('#app')
