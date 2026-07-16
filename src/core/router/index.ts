import { createRouter, createWebHashHistory } from 'vue-router'

export const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    {
      path: '/',
      name: 'search',
      component: () => import('@/modules/search/pages/SearchPage.vue'),
    },
    {
      path: '/settings',
      name: 'settings',
      component: () => import('@/modules/search/pages/SearchPage.vue'),
      meta: { isPanel: true },
    },
    {
      path: '/commands',
      name: 'commands',
      component: () => import('@/modules/search/pages/SearchPage.vue'),
      meta: { isPanel: true },
    },
  ],
})
