import { createRouter, createWebHashHistory } from 'vue-router'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      name: 'search',
      component: () => import('@/pages/SearchPage.vue'),
    },
    {
      path: '/startup',
      name: 'startup',
      component: () => import('@/pages/StartupPage.vue'),
    },
    {
      path: '/commands',
      name: 'commands',
      component: () => import('@/pages/CommandsPage.vue'),
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/pages/SettingsPage.vue'),
    },
  ],
})
