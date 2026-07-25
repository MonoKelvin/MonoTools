// ============================================================
// 搜索模块设置注册 (文件搜索)
// ============================================================

import { defineModuleSettings } from '@/modules/settings'

export const searchSettings = defineModuleSettings({
  moduleId: 'search',
  order: 10,
  groups: [
    {
      id: 'file-search',
      label: '文件搜索',
      icon: 'FolderSearch',
      order: 10,
      description: '使用 NTFS USN Journal 和 MFT 索引加速文件查找',
      items: [
        {
          key: 'fileSearchEnabled',
          type: 'boolean',
          label: '启用文件搜索',
          description: '开启后将构建文件名索引，提升搜索速度',
          default: true,
        },
        {
          key: 'fileSearchRoots',
          type: 'pathList',
          label: '搜索目录',
          description: '指定要索引的文件夹路径',
          default: [],
        },
      ],
    },
    {
      id: 'search-behavior',
      label: '搜索行为',
      icon: 'Search',
      order: 20,
      items: [
        {
          key: 'enabledCategories',
          type: 'select-multi',
          label: '搜索范围',
          description: '控制在哪些类别中搜索',
          default: ['apps', 'files', 'commands'],
          options: [
            { label: '应用程序', value: 'apps' },
            { label: '文件', value: 'files' },
            { label: '命令', value: 'commands' },
          ],
        },
      ],
    },
  ],
})
