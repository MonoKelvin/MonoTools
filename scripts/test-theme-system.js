/**
 * 测试主题加载和应用
 */

const fs = require('fs')
const path = require('path')

console.log('=== 主题系统测试 ===\n')

// 测试 1: 检查主题文件是否存在
console.log('1. 检查主题文件:')
const themes = ['linear-dark', 'vercel-light']

for (const themeName of themes) {
  const themeDir = path.join(__dirname, '..', 'internal-plugins', 'themes', themeName, 'public')
  const pluginJson = path.join(themeDir, 'plugin.json')
  const styleCss = path.join(themeDir, 'style.css')

  console.log(`\n  ${themeName}:`)
  console.log(`    目录: ${themeDir}`)
  console.log(`    目录存在: ${fs.existsSync(themeDir)}`)
  console.log(`    plugin.json 存在: ${fs.existsSync(pluginJson)}`)
  console.log(`    style.css 存在: ${fs.existsSync(styleCss)}`)

  if (fs.existsSync(pluginJson)) {
    const config = JSON.parse(fs.readFileSync(pluginJson, 'utf-8'))
    console.log(`    配置: id=${config.id}, type=${config.type}, themeType=${config.themeType}`)
  }
}

// 测试 2: 检查 style.css 是否包含 #app[data-theme="active"] 选择器
console.log('\n\n2. 检查 CSS 选择器包装:')
for (const themeName of themes) {
  const styleCss = path.join(
    __dirname,
    '..',
    'internal-plugins',
    'themes',
    themeName,
    'public',
    'style.css'
  )

  if (fs.existsSync(styleCss)) {
    const content = fs.readFileSync(styleCss, 'utf-8')
    const hasAppSelector = content.includes('#app[data-theme="active"]')
    const hasNestedApp = content.includes('#app[data-theme="active"] #app')

    console.log(`\n  ${themeName}:`)
    console.log(`    包含 #app[data-theme="active"]: ${hasAppSelector}`)
    console.log(`    包含嵌套 #app (错误): ${hasNestedApp}`)

    // 统计选择器数量
    const selectorCount = (content.match(/#app\[data-theme="active"\]/g) || []).length
    console.log(`    #app[data-theme="active"] 出现次数: ${selectorCount}`)
  }
}

// 测试 3: 检查 CSS 变量
console.log('\n\n3. 检查 CSS 变量定义:')
const requiredVars = [
  '--color-primary',
  '--color-on-primary',
  '--color-success',
  '--color-error',
  '--font-body',
  '--font-display',
  '--font-mono'
]

for (const themeName of themes) {
  const styleCss = path.join(
    __dirname,
    '..',
    'internal-plugins',
    'themes',
    themeName,
    'public',
    'style.css'
  )

  if (fs.existsSync(styleCss)) {
    const content = fs.readFileSync(styleCss, 'utf-8')

    console.log(`\n  ${themeName}:`)
    for (const varName of requiredVars) {
      const hasVar = content.includes(varName)
      console.log(`    ${varName}: ${hasVar ? '✅' : '❌'}`)
    }
  }
}

console.log('\n\n=== 测试完成 ===')
