import vue from '@vitejs/plugin-vue'
import { defineConfig } from 'vite'
import { fileURLToPath, URL } from 'node:url'

// https://vitejs.dev/config/
export default defineConfig({
  base: './',
  server: {
    port: 5177, // 设置插件开发服务器端口（避免与主程序 5173 冲突）
    strictPort: true,
    open: false
  },
  plugins: [
    vue(),
    // UnoCSS() - 注释掉以避免依赖问题
    // {
    //   name: 'uno-css',
    //   enforce: 'pre',
    //   transformIndexHtml(html) {
    //     return html.replace(/<head>([\s\S]*)<\/head>/, '<head><style type="text/css">/* UnoCSS styles will be injected here */</style>$1</head>')
    //   }
    // }
    {
      name: 'configure-response-headers',
      configureServer: (server) => {
        server.middlewares.use((_req, res, next) => {
          // 允许加载自定义协议（monotools-icon://）和本地文件（file://）
          res.setHeader(
            'Content-Security-Policy',
            "default-src * 'unsafe-inline' 'unsafe-eval' data: blob: monotools-icon: file:; img-src * data: blob: monotools-icon: file:;"
          )
          next()
        })
      }
    }
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '@shared': fileURLToPath(new URL('../../src/shared', import.meta.url))
    }
  }
})
