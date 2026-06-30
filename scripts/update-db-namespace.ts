#!/usr/bin/env node

/**
 * 批量更新数据库命名空间从 MONOTOOLS/ 到 MONOTOOLS/
 */

const fs = require('fs')
const path = require('path')

/**
 * 递归查找所有文件
 */
function findFiles(dir, extensions = ['.ts', '.js', '.vue', '.html']) {
  const files = []
  const items = fs.readdirSync(dir, { withFileTypes: true })

  for (const item of items) {
    const fullPath = path.join(dir, item.name)

    if (item.isDirectory()) {
      if (
        item.name !== 'node_modules' &&
        item.name !== '.git' &&
        item.name !== 'dist' &&
        item.name !== 'out'
      ) {
        files.push(...findFiles(fullPath, extensions))
      }
    } else if (item.isFile() && extensions.some((ext) => item.name.endsWith(ext))) {
      files.push(fullPath)
    }
  }

  return files
}

/**
 * 处理单个文件
 */
function processFile(filePath) {
  try {
    let content = fs.readFileSync(filePath, 'utf-8')
    let changes = 0

    // 替换 MONOTOOLS/ 为 MONOTOOLS/
    if (content.includes('MONOTOOLS/')) {
      content = content.replace(/ZTOOLS\//g, 'MONOTOOLS/')
      changes++
    }

    // 保存更改
    if (changes > 0) {
      fs.writeFileSync(filePath, content, 'utf-8')
      return changes
    }

    return 0
  } catch (error) {
    console.error(`处理文件失败 ${filePath}:`, error.message)
    return 0
  }
}

/**
 * 处理单个目录
 */
function processDirectory(dir) {
  console.log(`\n处理目录: ${dir}`)
  console.log('─'.repeat(60))

  const files = findFiles(dir)
  console.log(`找到 ${files.length} 个文件`)

  let totalChanges = 0

  for (const file of files) {
    const changes = processFile(file)
    if (changes > 0) {
      console.log(`  ✓ ${file} (${changes} 处更改)`)
      totalChanges += changes
    }
  }

  return totalChanges
}

/**
 * 主函数
 */
function main() {
  console.log('═══════════════════════════════════════════════════')
  console.log('   更新数据库命名空间: MONOTOOLS/ → MONOTOOLS/')
  console.log('═══════════════════════════════════════════════════\n')

  // 要处理的目录
  const dirs = [
    path.join(__dirname, '..'),
    path.join(__dirname, '..', 'src'),
    path.join(__dirname, '..', 'internal-plugins'),
    path.join(__dirname, '..', 'resources')
  ]

  let totalChanges = 0

  for (const dir of dirs) {
    if (fs.existsSync(dir)) {
      totalChanges += processDirectory(dir)
    }
  }

  console.log('─'.repeat(60))
  console.log(`\n总计: ${totalChanges} 处更改`)
  console.log('\n═══════════════════════════════════════════════════')
}

main()
