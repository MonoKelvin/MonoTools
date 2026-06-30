import lmdbInstance from './lmdb/lmdbInstance'

/**
 * 数据库命名空间迁移
 * ZTOOLS/ → MONOTOOLS/
 */
export async function migrateDatabaseNamespaces(): Promise<void> {
  console.log('[Migration] 开始数据库命名空间迁移...')

  try {
    // 检查是否已有 MONOTOOLS 数据（避免重复迁移）
    const existingDocs = await lmdbInstance.promises.allDocs('MONOTOOLS')

    if (existingDocs.length > 0) {
      console.log(`[Migration] MONOTOOLS 命名空间已有 ${existingDocs.length} 个文档，无需迁移`)
      return
    }

    // 检查是否有 ZTOOLS 数据需要迁移
    const ztoolsDocs = await lmdbInstance.promises.allDocs('ZTOOLS')

    if (ztoolsDocs.length === 0) {
      console.log('[Migration] ZTOOLS 命名空间为空，无需迁移')
      return
    }

    console.log(`[Migration] 找到 ${ztoolsDocs.length} 个 ZTOOLS 文档，准备迁移...`)

    // 迁移每个文档（保留 _rev 以确保更新操作能正确执行）
    for (const doc of ztoolsDocs) {
      // 检查是否已经迁移过
      const existingMonotoolsDoc = await lmdbInstance.promises.get(doc._id)
      if (existingMonotoolsDoc) {
        console.log(`[Migration] 文档 ${doc._id} 已存在，跳过`)
        continue
      }

      // 直接读取原始文档（包含原始 _rev）
      const originalDoc = await lmdbInstance.promises.get(`ZTOOLS/${doc._id}`)
      if (originalDoc) {
        // 创建新文档，使用 MONOTOOLS 命名空间
        await lmdbInstance.promises.put({
          _id: `MONOTOOLS/${doc._id}`,
          _rev: originalDoc._rev,
          data: originalDoc.data
        })
        console.log(`[Migration] 迁移文档: ZTOOLS/${doc._id} → MONOTOOLS/${doc._id}`)
      }
    }

    console.log('[Migration] 数据库命名空间迁移完成')
  } catch (error) {
    console.error('[Migration] 迁移失败:', error)
    throw error
  }
}
