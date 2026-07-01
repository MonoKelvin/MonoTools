/**
 * CSS主题包装脚本
 * 为所有CSS选择器添加 #app[data-theme="active"] 前缀以提高优先级
 */

const fs = require('fs')
const path = require('path')

/**
 * 为单个CSS行添加选择器前缀
 */
function wrapSelectorLine(line) {
  const braceIndex = line.indexOf('{')
  if (braceIndex === -1) return line

  const selector = line.substring(0, braceIndex).trim()
  const declaration = line.substring(braceIndex)

  // 处理多个选择器（逗号分隔）
  const selectors = selector
    .split(',')
    .map((s) => s.trim())
    .filter((s) => s.length > 0)
    .map((s) => {
      // 跳过 :root（不需要包装）
      if (s === ':root') return s
      // 跳过 html[...]（已经包装过）
      if (s.startsWith('html[')) return s
      // 跳过 #app[...]（已经包装过）
      if (s.startsWith('#app[')) return s
      // 跳过 :root 后的全局选择器
      if (s === 'body' || s === 'html') return s
      // 为其他选择器添加 #app[data-theme="active"] 前缀
      return `#app[data-theme="active"] ${s}`
    })
    .join(', ')

  return selectors + declaration
}

/**
 * 包装CSS内容
 */
function wrapCSS(cssContent) {
  const lines = cssContent.split('\n')
  const result = []
  let i = 0

  while (i < lines.length) {
    const line = lines[i]
    const trimmed = line.trim()

    // 跳过注释和空行
    if (
      !trimmed ||
      trimmed.startsWith('/*') ||
      trimmed.startsWith('*') ||
      trimmed.startsWith('//')
    ) {
      result.push(line)
      i++
      continue
    }

    // 检测选择器行（包含 { 的行）
    if (trimmed.includes('{')) {
      const wrappedLine = wrapSelectorLine(line)
      result.push(wrappedLine)
      i++
      continue
    }

    // 其他行原样保留
    result.push(line)
    i++
  }

  return result.join('\n')
}

// 处理 linear-dark 主题
const linearDarkPath = path.join(
  __dirname,
  '..',
  'internal-plugins',
  'themes',
  'linear-dark',
  'public',
  'style.css'
)
const linearDarkCSS = fs.readFileSync(linearDarkPath, 'utf-8')
const wrappedLinearDark = wrapCSS(linearDarkCSS)
fs.writeFileSync(linearDarkPath, wrappedLinearDark)
console.log('✅ linear-dark/public/style.css 已更新')

// 处理 vercel-light 主题
const vercelLightPath = path.join(
  __dirname,
  '..',
  'internal-plugins',
  'themes',
  'vercel-light',
  'public',
  'style.css'
)
const vercelLightCSS = fs.readFileSync(vercelLightPath, 'utf-8')
const wrappedVercelLight = wrapCSS(vercelLightCSS)
fs.writeFileSync(vercelLightPath, wrappedVercelLight)
console.log('✅ vercel-light/public/style.css 已更新')

console.log('\n✅ 所有主题CSS已更新，选择器优先级已提升')
