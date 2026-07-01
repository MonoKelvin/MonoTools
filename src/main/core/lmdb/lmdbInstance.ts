import { app } from 'electron'
import path from 'path'
import LmdbDatabase from './index'

/**
 * 创建共享的 LMDB 数据库实例
 * 数据库文件存储在 userData/lmdb 目录下
 */
const lmdbInstance = new LmdbDatabase({
  path: path.join(app.getPath('userData'), 'lmdb'),
  mapSize: 2 * 1024 * 1024 * 1024, // 2GB
  maxDbs: 3 // main, meta, attachment
})

console.log('[LMDB] LMDB database created successfully')
console.log('[LMDB] Database instance ID:', lmdbInstance)

// 导出单例实例
export default lmdbInstance

// 跟踪数据库关闭状态
let isLmdbClosed = false
let lmdbCloseCallStack = ''

/**
 * 清理函数：应用退出时调用
 */
export function closeLmdb(): void {
  if (isLmdbClosed) {
    console.warn('[LMDB] Database already closed! Ignoring duplicate close call.')
    console.warn('[LMDB] Previous close call stack:', lmdbCloseCallStack)
    console.warn('[LMDB] Current close call stack:', new Error().stack)
    return
  }

  try {
    isLmdbClosed = true
    lmdbCloseCallStack = new Error().stack || 'No stack trace available'
    console.log('[LMDB] Closing LMDB database...')
    console.log('[LMDB] Close called from:', lmdbCloseCallStack)
    lmdbInstance.close()
    console.log('[LMDB] LMDB database closed successfully')
  } catch (e) {
    console.error('[LMDB] Error closing LMDB:', e)
  }
}

// 导出原始 close 方法供特殊场景使用
export function forceCloseLmdb(): void {
  if (isLmdbClosed) {
    console.warn('[LMDB] Database already closed!')
    return
  }

  try {
    isLmdbClosed = true
    lmdbCloseCallStack = new Error().stack || 'No stack trace available'
    console.log('[LMDB] Force closing LMDB database...')
    console.log('[LMDB] Force close called from:', lmdbCloseCallStack)
    lmdbInstance.close()
    console.log('[LMDB] LMDB database force closed successfully')
  } catch (e) {
    console.error('[LMDB] Error force closing LMDB:', e)
  }
}

// 监听应用退出事件，自动关闭数据库
app.on('will-quit', () => {
  closeLmdb()
})
