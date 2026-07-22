/**
 * 图标渲染配置 —— 集中管理所有图标相关的"魔法数".
 *
 * 修改这里一处, 全工程 (前端 useAppIcon / useIconRenderer / AppResultItem /
 * ResultItem / lobehubIcons 等) 同步生效.
 *
 * 跨前后端同步: 改本文件须同步改 `src-tauri/src/config.rs::icon::*`.
 * 详见 CLAUDE.md "编码规范" 1.3.
 */

export const ICON_CONFIG = {
    /**
     * 图标像素尺寸. 与后端 `config::icon::SIZE` 对齐, 两者必须一致.
     * 128x128 已经足够清晰, 显著降低 PNG 体积和提取时间.
     *
     * 后端提取策略 (`platform/windows/icon.rs::extract_icon_windows`)
     * 按"高分辨率原生 → 高质量缩放"4-tier 优先级:
     * - Tier 1: `SHGetImageList(SHIL_JUMBO)` + `IImageList2::GetIcon` — Vista+
     *   系统 256x256 图像列表, 与 Windows 资源管理器任务栏同一来源; Win 8.1+
     *   多数应用注册了真正的 256x256, 这一步直接拿到原生 128x128 HICON, 0 锯齿.
     * - Tier 2: `IShellItemImageFactory::GetImage(SIIGBF_ICONONLY)` — 不带
     *   `SIIGBF_BIGGERSIZEOK`, 严格只接受原生命尺寸位图, 避免强制放大.
     * - Tier 3/4: `ExtractIconExW` / `SHGetFileInfoW` + `SetStretchBltMode(HALFTONE)`
     *   高质量 GDI 拉伸, 避免默认 nearest-neighbor 锯齿.
     */
    size: 128,

    /**
     * imgReady 兜底 timer (ms).
     *
     * 为什么需要: 在以下场景 `<img>` 的 @load 事件可能丢失, 导致 imgReady
     * 永远 = false → opacity 0 → 看上去是空白:
     * - 虚拟列表 v-for 复用 DOM, src 字符串相同时 Chromium 短路不发 load
     * - WebView2 内部 image state machine 与 DOM patch 顺序耦合
     * - happy-dom 测试环境根本不实现 img loading
     * - 缓存命中 + 相同 IconState 引用, Vue setter 跳过更新
     *
     * 因此每次进入 png/svg 路径时启动该 timer. 若 @load 在这之前已触发,
     * 提前 clearTimeout 取消兜底. 若 @load 丢失, 兜底强制显示.
     *
     * 500ms 经验值: 弱机 256x256 PNG 解码一般 < 100ms, 500ms 已足够覆盖
     * 99% 场景, 又不会让用户觉得"等太久".
     */
    loadFallbackMs: 500,

    /**
     * 后端 base64 最短长度 (字符数).
     * 128x128 RGBA PNG 压缩后通常 500-2000 chars, 128 是安全下限:
     * 低于此长度一定不是合法 PNG (PNG magic iVBORw0KGgo 自身就 12 chars
     * + 必然的 IDAT/IEND header). 用于 useAppIcon 在拼 data URL 前做严格
     * 校验, 避免 Chromium 静默吞掉"假成功"的短 base64.
     */
    minBase64Length: 128,

    /**
     * PNG magic 89 50 4E 47 0D 0A 1A 0A 的 base64 编码.
     *
     * 用于 useAppIcon 校验后端返回的真的是 PNG 而非损坏数据.
     * 与后端 `config::icon::PNG_MAGIC_BASE64` 对齐.
     */
    pngMagicBase64: 'iVBORw0KGgo',

    /**
     * icon opacity 渐入动画 (ms).
     *
     * 与 SCSS `.app-result-item__img { transition: opacity 220ms ... }`
     * 同步, 改这里同步改 SCSS.
     */
    fadeInMs: 220,

    /**
     * ResultItem tooltip showDelay (ms). 用于浮窗显示延迟.
     * 适当增加延迟，避免鼠标滑过时过于频繁地触发。
     */
    tooltipDelayMs: 500,

    /**
     * AppResultItem tooltip showDelay (ms).
     * 略长于 ResultItem, 避免与列表抖动重叠.
     */
    appTooltipDelayMs: 600,
} as const

export type IconConfig = typeof ICON_CONFIG
