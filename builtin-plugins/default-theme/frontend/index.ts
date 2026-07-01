// 默认主题插件激活
export function onActivate(ctx) {
  console.log('Default theme activated')

  // 加载主题 CSS
  const style = document.createElement('link')
  style.rel = 'stylesheet'
  style.href = chrome.runtime.getURL('theme.css')
  document.head.appendChild(style)

  // 注册主题提供者
  ctx.theme?.register?.({
    id: 'builtin:default-theme',
    name: '默认深色',
    modes: ['dark', 'light', 'system'],
    defaultMode: 'dark'
  })
}
