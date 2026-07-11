// vitest 配置：CJS path.resolve 可让 vite 正确解析 '@/...' 等 alias
import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import path from 'node:path'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, 'src'),
    },
  },
  test: {
    environment: 'happy-dom',
    // 扫描 tests/ui 下面的用例；tests/common 留给 Rust 共享，UI 不要混入
    include: ['tests/ui/**/*.test.ts'],
    globals: true,
    clearMocks: true,
    restoreMocks: true,
  },
})
