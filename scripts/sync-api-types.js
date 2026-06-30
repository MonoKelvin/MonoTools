#!/usr/bin/env node
/* eslint-disable @typescript-eslint/no-require-imports */
/**
 * 从 uTools API Types 同步并转换为 MonoTools API Types
 *
 * 用法：
 *   node scripts/sync-api-types.js
 */

const { execSync } = require('child_process')
const fs = require('fs-extra')
const path = require('path')

const UTOOLS_REPO = 'https://github.com/uTools-Labs/utools-api-types.git'
const SOURCE_DIR = path.join(__dirname, '../.monotools-api-types-source')
const TARGET_DIR = path.join(__dirname, '../monotools-api-types')

// 替换规则
const REPLACEMENTS = [
  // API 名称和接口名
  { from: /window\.utools/g, to: 'window.monotools' },
  { from: /UToolsAPI/g, to: 'MonoToolsAPI' },
  { from: /UToolsApi/g, to: 'MonoToolsApi' }, // 注意大小写变体
  { from: /UtoolsAi/g, to: 'MonoToolsAi' }, // 接口名前缀
  { from: /utools-api-types/g, to: 'monotools-api-types' },

  // 变量声明
  { from: /declare var utools:/g, to: 'declare var monotools:' },
  { from: /export = utools/g, to: 'export = monotools' },

  // 文件名引用
  { from: /ubw\.d\.ts/g, to: 'zbw.d.ts' },
  { from: /utools\.api\.d\.ts/g, to: 'monotools.api.d.ts' },
  { from: /resource\/utools\.schema/g, to: 'resource/monotools.schema' },
  { from: /utools\.schema\.json/g, to: 'monotools.schema.json' },

  // URL 和链接
  {
    from: /github\.com\/uTools-Labs\/utools-api-types/g,
    to: 'github.com/MonoToolsCenter/MonoTools/tree/main/monotools-api-types'
  },

  // 注释和文档中的 utools（小写）
  { from: /将 utools 挂载/g, to: '将 monotools 挂载' },
  { from: /ubrowserId/g, to: 'zbrowserId' },
  { from: /ubrowser/g, to: 'zbrowser' },
  { from: /utools\./g, to: 'monotools.' }, // 方法调用 utools.xxx() -> monotools.xxx()

  // 组织和项目名称
  { from: /uTools-Labs/gi, to: 'MonoTools-Labs' },

  // 描述文本（但保留一些特定的）
  { from: /\buTools\b/g, to: 'MonoTools' }
]

// 需要转换的文件列表
const FILES_TO_CONVERT = [
  'index.d.ts',
  'utools.api.d.ts', // 会被重命名为 monotools.api.d.ts
  'ubw.d.ts', // 会被重命名为 zbw.d.ts
  'electron.d.ts',
  'resource/utools.schema.json', // 会被重命名为 monotools.schema.json
  'package.json',
  'tsconfig.json',
  'README.md',
  'LICENSE'
]

console.log('🚀 开始同步 uTools API Types...\n')

try {
  // Step 1: 克隆或更新 uTools 仓库
  console.log('📥 Step 1: 拉取 uTools 源码...')
  if (fs.existsSync(SOURCE_DIR)) {
    console.log('   源码目录已存在，更新中...')
    execSync('git pull', { cwd: SOURCE_DIR, stdio: 'inherit' })
  } else {
    console.log('   克隆仓库中...')
    execSync(`git clone ${UTOOLS_REPO} "${SOURCE_DIR}"`, { stdio: 'inherit' })
  }

  // 获取 uTools 版本信息
  const utoolsPackageJson = fs.readJsonSync(path.join(SOURCE_DIR, 'package.json'))
  const utoolsVersion = utoolsPackageJson.version || 'unknown'
  const gitCommit = execSync('git rev-parse --short HEAD', { cwd: SOURCE_DIR }).toString().trim()

  console.log(`   uTools 版本: ${utoolsVersion} (commit: ${gitCommit})\n`)

  // Step 2: 创建目标目录
  console.log('📁 Step 2: 准备目标目录...')
  fs.ensureDirSync(TARGET_DIR)
  fs.ensureDirSync(path.join(TARGET_DIR, 'resource'))

  // Step 3: 复制并转换文件
  console.log('🔄 Step 3: 转换文件...')
  FILES_TO_CONVERT.forEach((file) => {
    const sourcePath = path.join(SOURCE_DIR, file)

    if (!fs.existsSync(sourcePath)) {
      console.log(`   ⚠️  跳过不存在的文件: ${file}`)
      return
    }

    // 确定目标文件名（处理重命名）
    let targetFile = file
    if (file === 'utools.api.d.ts') targetFile = 'monotools.api.d.ts'
    if (file === 'ubw.d.ts') targetFile = 'zbw.d.ts'
    if (file === 'resource/utools.schema.json') targetFile = 'resource/monotools.schema.json'

    const targetPath = path.join(TARGET_DIR, targetFile)

    console.log(`   转换: ${file} -> ${targetFile}`)

    // 读取文件内容
    let content = fs.readFileSync(sourcePath, 'utf-8')

    // 应用所有替换规则
    REPLACEMENTS.forEach((rule) => {
      content = content.replace(rule.from, rule.to)
    })

    // 写入目标文件
    fs.writeFileSync(targetPath, content, 'utf-8')
  })

  // Step 4: 创建/更新 MonoTools 特定文件
  console.log('\n📝 Step 4: 创建 MonoTools 特定文件...')

  // 更新 package.json
  const packageJson = {
    name: '@monotools-center/monotools-api-types',
    version: '1.0.0',
    description: 'MonoTools API 代码提示 （直接Copy uTools的，部分API未完成适配）',
    main: 'index.d.ts',
    types: 'index.d.ts',
    files: [
      'index.d.ts',
      'monotools.api.d.ts',
      'zbw.d.ts',
      'electron.d.ts',
      'resource/',
      'README.md',
      'API_STATUS.md',
      'LICENSE'
    ],
    keywords: ['monotools', 'utools', 'plugin', 'typescript', 'types', 'api'],
    author: 'MonoTools Contributors',
    license: 'MIT',
    repository: {
      type: 'git',
      url: 'https://github.com/MonoToolsCenter/MonoTools.git',
      directory: 'monotools-api-types'
    },
    bugs: {
      url: 'https://github.com/MonoToolsCenter/MonoTools/issues'
    },
    homepage: 'https://github.com/MonoToolsCenter/MonoTools/tree/main/monotools-api-types',
    // 记录同步信息
    'utools-version': utoolsVersion,
    'utools-commit': gitCommit,
    'sync-date': new Date().toISOString().split('T')[0]
  }

  fs.writeJsonSync(path.join(TARGET_DIR, 'package.json'), packageJson, { spaces: 2 })
  console.log('   ✅ package.json')

  // 创建 .npmignore
  const npmignore = `*.log
node_modules/
.DS_Store
test/
examples/
tsconfig.json
SYNC_GUIDE.md
.git
`
  fs.writeFileSync(path.join(TARGET_DIR, '.npmignore'), npmignore)
  console.log('   ✅ .npmignore')

  // 创建 API_STATUS.md
  const apiStatus = `# MonoTools API 实现状态

> 本文档由脚本自动生成，同步自 utools-api-types@${utoolsVersion}

## ✅ 已完全实现

### 平台检测
- \`isMacOs()\` / \`isMacOS()\`
- \`isWindows()\`
- \`isLinux()\`
- \`isDarkColors()\`

### 插件生命周期
- \`onPluginEnter(callback)\`
- \`onPluginReady(callback)\`
- \`outPlugin(isKill?)\`

### UI 控制
- \`showNotification(body)\`
- \`setExpendHeight(height)\`
- \`setSubInput(onChange?, placeholder?, isFocus?)\`
- \`setSubInputValue(text)\`
- \`subInputFocus()\`

### 窗口操作
- \`hideMainWindow(isRestorePreWindow?)\`
- \`createBrowserWindow(url, options?, callback?)\`

### 数据库 API
- \`db.*\` - 完整的同步/异步 API
- \`dbStorage.*\` - 类 localStorage API

### 动态 Feature API
- \`getFeatures(codes?)\`
- \`setFeature(feature)\`
- \`removeFeature(code)\`

### 剪贴板 API
- \`clipboard.*\` - 完整的剪贴板管理 API
- \`copyText(text)\`, \`copyImage(image)\`, \`copyFile(filePath)\`

### 系统对话框
- \`getPath(name)\`, \`showSaveDialog(options)\`, \`showOpenDialog(options)\`

### 屏幕功能
- \`screenCapture(callback?)\`
- \`getPrimaryDisplay()\`, \`getAllDisplays()\`
- 其他屏幕 API

### 其他工具
- \`sendInputEvent(event)\`
- \`shellOpenExternal(url)\`, \`shellShowItemInFolder(fullPath)\`
- \`redirect(label, payload?)\`
- 等等...

## ⏳ 计划实现

### ubrowser API（高优先级）
- 网页自动化完整 API

## 📝 兼容性说明

MonoTools 致力于 100% 兼容 uTools 插件 API。未实现的 API 仍保留在类型定义中。

---

**同步信息**
- uTools 版本: ${utoolsVersion}
- Git Commit: ${gitCommit}
- 同步日期: ${packageJson['sync-date']}
`
  fs.writeFileSync(path.join(TARGET_DIR, 'API_STATUS.md'), apiStatus)
  console.log('   ✅ API_STATUS.md')

  // Step 5: 完成
  console.log('\n✅ 同步完成！\n')
  console.log('📊 同步信息:')
  console.log(`   - uTools 版本: ${utoolsVersion}`)
  console.log(`   - Git Commit: ${gitCommit}`)
  console.log(`   - 同步日期: ${packageJson['sync-date']}`)
  console.log(`\n📁 输出目录: ${TARGET_DIR}`)
  console.log('\n💡 下一步:')
  console.log('   1. 检查生成的文件')
  console.log('   2. 运行测试验证')
  console.log('   3. 如需发布: cd monotools-api-types && npm publish\n')
} catch (error) {
  console.error('\n❌ 同步失败:', error.message)
  process.exit(1)
}
