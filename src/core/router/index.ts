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
      path: '/settings',
      name: 'settings',
      component: () => import('@/pages/SearchPage.vue'),
      meta: { isPanel: true },
    },
    {
      path: '/commands',
      name: 'commands',
      component: () => import('@/pages/SearchPage.vue'),
      meta: { isPanel: true },
    },
  ],
})
