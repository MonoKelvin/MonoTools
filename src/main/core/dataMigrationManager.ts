import fs from 'fs';
import path from 'path';
import log from 'electron-log';
import lmdbInstance from './lmdb/lmdbInstance';
import { app } from 'electron';

/**
 * 数据迁移管理器
 * 负责将 ZTools 的数据迁移到 MonoTools
 */
export class DataMigrationManager {
  private userDataPath: string;
  private ztoolsPaths: {
    data: string;
    config: string;
    clipboard: string;
    plugins: string;
  };

  constructor() {
    this.userDataPath = app.getPath('userData');
    this.ztoolsPaths = {
      data: path.join(this.userDataPath, 'lmdb.ztools'),
      config: path.join(this.userDataPath, 'settings.ztools.json'),
      clipboard: path.join(this.userDataPath, 'clipboard.ztools.json'),
      plugins: path.join(this.userDataPath, 'plugins.ztools')
    };
  }

  /**
   * 检测是否需要迁移
   */
  needsMigration(): boolean {
    return fs.existsSync(this.ztoolsPaths.data) ||
           fs.existsSync(this.ztoolsPaths.config) ||
           fs.existsSync(this.ztoolsPaths.clipboard) ||
           fs.existsSync(this.ztoolsPaths.plugins);
  }

  /**
   * 迁移所有数据
   */
  async migrate(): Promise<boolean> {
    try {
      log.info('[DataMigration] 开始迁移 ZTools 数据...');

      // 1. 迁移剪贴板历史
      await this.migrateClipboardHistory();

      // 2. 迁移配置文件
      await this.migrateConfigFiles();

      // 3. 迁移插件
      await this.migratePlugins();

      // 4. 迁移 LMDB 数据
      await this.migrateLmdbData();

      log.info('[DataMigration] 迁移完成');
      return true;
    } catch (error) {
      log.error('[DataMigration] 迁移失败:', error);
      return false;
    }
  }

  /**
   * 迁移剪贴板历史
   */
  private async migrateClipboardHistory(): Promise<void> {
    const source = this.ztoolsPaths.clipboard;
    const target = path.join(this.userDataPath, 'clipboard.json');

    if (!fs.existsSync(source)) {
      log.info('[DataMigration] 未找到剪贴板历史，跳过');
      return;
    }

    try {
      log.info('[DataMigration] 迁移剪贴板历史:', source);
      fs.copyFileSync(source, target);
      log.info('[DataMigration] ✅ 剪贴板历史迁移完成');
    } catch (error) {
      log.error('[DataMigration] ❌ 剪贴板历史迁移失败:', error);
      throw error;
    }
  }

  /**
   * 迁移配置文件
   */
  private async migrateConfigFiles(): Promise<void> {
    const source = this.ztoolsPaths.config;
    const target = path.join(this.userDataPath, 'settings.json');

    if (!fs.existsSync(source)) {
      log.info('[DataMigration] 未找到配置文件，跳过');
      return;
    }

    try {
      log.info('[DataMigration] 迁移配置文件:', source);

      // 读取并更新配置
      const config = JSON.parse(fs.readFileSync(source, 'utf-8'));

      // 更新应用 ID（如果存在）
      if (config.appId === 'link.eiot.ztools') {
        config.appId = 'link.eiot.monotools';
        log.info('[DataMigration] 更新应用 ID');
      }

      // 更新产品名称
      if (config.productName === 'ZTools') {
        config.productName = 'MonoTools';
        log.info('[DataMigration] 更新产品名称');
      }

      // 写入新配置
      fs.writeFileSync(target, JSON.stringify(config, null, 2));
      log.info('[DataMigration] ✅ 配置文件迁移完成');
    } catch (error) {
      log.error('[DataMigration] ❌ 配置文件迁移失败:', error);
      throw error;
    }
  }

  /**
   * 迁移插件目录
   */
  private async migratePlugins(): Promise<void> {
    const source = this.ztoolsPaths.plugins;
    const target = path.join(this.userDataPath, 'plugins');

    if (!fs.existsSync(source)) {
      log.info('[DataMigration] 未找到插件目录，跳过');
      return;
    }

    try {
      log.info('[DataMigration] 迁移插件目录:', source);

      // 清空目标目录
      if (fs.existsSync(target)) {
        fs.rmSync(target, { recursive: true, force: true });
      }

      // 创建目标目录
      fs.mkdirSync(target, { recursive: true });

      // 复制所有文件
      this.copyDirectory(source, target);

      log.info('[DataMigration] ✅ 插件目录迁移完成');
    } catch (error) {
      log.error('[DataMigration] ❌ 插件目录迁移失败:', error);
      throw error;
    }
  }

  /**
   * 迁移 LMDB 数据
   * 注意：需要应用先退出，迁移后重启
   */
  private async migrateLmdbData(): Promise<void> {
    const source = this.ztoolsPaths.data;
    const target = path.join(this.userDataPath, 'lmdb');

    if (!fs.existsSync(source)) {
      log.info('[DataMigration] 未找到 LMDB 数据，跳过');
      return;
    }

    try {
      log.info('[DataMigration] 迁移 LMDB 数据（需要重启）');

      // 创建目标目录
      if (!fs.existsSync(target)) {
        fs.mkdirSync(target, { recursive: true });
      }

      // 使用 child_process 执行迁移（避免主进程独占访问）
      const { execSync } = require('child_process');

      // Windows: 使用 copy 命令
      if (process.platform === 'win32') {
        execSync(
          `xcopy "${source}\\*" "${target}\\*" /E /I /H /Y`,
          { stdio: 'inherit' }
        );
      } else {
        // macOS/Linux: 使用 cp 命令
        execSync(`cp -r "${source}"/* "${target}/"`, { stdio: 'inherit' });
      }

      log.info('[DataMigration] ✅ LMDB 数据迁移完成');
    } catch (error: any) {
      if (error.code === 'EPERM' || error.code === 'EBUSY') {
        log.error('[DataMigration] ❌ LMDB 数据正在使用，迁移失败');
        throw new Error('数据正在使用中，请关闭所有 MonoTools 实例后再试');
      }
      log.error('[DataMigration] ❌ LMDB 数据迁移失败:', error.message);
      throw error;
    }
  }

  /**
   * 清理旧数据
   */
  async cleanupOldData(): Promise<void> {
    try {
      log.info('[DataMigration] 清理旧数据...');

      // 删除旧数据目录
      if (fs.existsSync(this.ztoolsPaths.data)) {
        fs.rmSync(this.ztoolsPaths.data, { recursive: true, force: true });
        log.info('[DataMigration] ✅ 旧 LMDB 数据已删除');
      }

      if (fs.existsSync(this.ztoolsPaths.config)) {
        fs.unlinkSync(this.ztoolsPaths.config);
        log.info('[DataMigration] ✅ 旧配置文件已删除');
      }

      if (fs.existsSync(this.ztoolsPaths.clipboard)) {
        fs.unlinkSync(this.ztoolsPaths.clipboard);
        log.info('[DataMigration] ✅ 旧剪贴板文件已删除');
      }

      if (fs.existsSync(this.ztoolsPaths.plugins)) {
        fs.rmSync(this.ztoolsPaths.plugins, { recursive: true, force: true });
        log.info('[DataMigration] ✅ 旧插件目录已删除');
      }
    } catch (error) {
      log.warn('[DataMigration] 清理旧数据失败（可能被占用）:', error);
    }
  }

  /**
   * 复制目录
   */
  private copyDirectory(source: string, target: string): void {
    const items = fs.readdirSync(source);

    for (const item of items) {
      const sourcePath = path.join(source, item);
      const targetPath = path.join(target, item);
      const stats = fs.statSync(sourcePath);

      if (stats.isDirectory()) {
        fs.mkdirSync(targetPath, { recursive: true });
        this.copyDirectory(sourcePath, targetPath);
      } else {
        fs.copyFileSync(sourcePath, targetPath);
      }
    }
  }
}
