import { createApp } from 'vue'
import App from './App.vue'

import './assets/styles/main.scss'

import PrimeVue from 'primevue/config'
import Aura from '@primeuix/themes/aura'
import Tooltip from 'primevue/tooltip'

import { router } from './router'
import { pinia } from './stores'
import { useAppIcon } from './composables/useAppIcon'
import { dumpIconTraceSummary, resetIconTrace } from './stores/iconLog'

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

/**
 * 开发期图标诊断快捷方式 (DevTools 控制台):
 *   window.__iconDebug.enable()  // 打开 trace/failure 日志
 *   window.__iconDebug.disable() // 关闭
 *   window.__iconDebug.dump()    // 打印当前命中率 + 最近 10 条 trace
 *   window.__iconDebug.reset()   // 重置计数
 *
 * 生产环境 (import.meta.env.PROD === true) 完全不挂这个 global,
 * 不污染线上 bundle 的运行时.
 */
if (import.meta.env.DEV) {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  ;(window as any).__iconDebug = {
    enable: () => useAppIcon().setDebug(true),
    disable: () => useAppIcon().setDebug(false),
    dump: () => {
      dumpIconTraceSummary()
    },
    reset: () => {
      resetIconTrace()
    },
  }
  // eslint-disable-next-line no-console
  console.info(
    '[icon-debug] window.__iconDebug 已挂载; ' +
      '在控制台调用 __iconDebug.dump() 查看图标加载分布.',
  )
}

app.mount('#app')
