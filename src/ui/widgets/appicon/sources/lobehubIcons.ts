/**
 * LobeHub UI 彩色图标 (CDN)
 *
 * 在 useAppIcon 的 3 级加载链中作为"中间级"插入:
 *   1. 静态命中 (Lucide 组件)  -> 0ms
 *   2. ★ LobeHub 模糊匹配 (彩色 SVG) -> ~50-200ms (CDN)
 *   3. 后端 IPC 提取 PNG -> 异步 IPC, 50-500ms
 *   4. 通用兜底 Lucide
 *
 * 设计原则:
 *   - **永不抛错** — 任何网络/解析失败都返回 null, 让上层继续降级
 *   - **快速失败** — 5s 超时, 避免在弱网下卡住整个图标链
 *   - **CORS 友好** — lobehub.com 默认允许 CORS, 直接 fetch 即可
 *   - **零额外依赖** — 浏览器原生 fetch + atob/blob, 不引入第三方 HTTP 库
 *
 * URL 格式 (2026-07 验证):
 *   - `https://lobehub.com/icon-colorful/{slug}` 彩色 SVG
 *   - `https://lobehub.com/icon/{slug}`           单色 SVG
 *   - 内容: `<svg viewBox="0 0 24 24">...</svg>`, 体积 ~1-3KB
 *
 * Slug 提取策略 (模糊匹配):
 *   1. 直接用 `name` 试一次 (e.g., "WeChat" -> "wechat")
 *   2. 失败则逐个去掉空格/特殊字符, 试短 token (e.g., "Microsoft Word" -> "word")
 *   3. 最后用文件路径中的可执行文件名 (无 .exe) 试一次
 *   4. 3 次都不命中 -> 返回 null
 */

import type { IconState } from './types'

const LOBEHUB_TIMEOUT_MS = 5000

/**
 * 把任意 app name 规整成 lobehub 期望的 slug.
 * - 全小写
 * - 非 a-z0-9- 替换为 -
 * - 去除首尾 -
 * - 限制长度 1-32
 */
export function toLobehubSlug(input: string): string {
    return input
        .toLowerCase()
        .replace(/[^a-z0-9-]+/g, '-')
        .replace(/^-+|-+$/g, '')
        .slice(0, 32)
}

/**
 * 候选 slug 列表 (按优先级从高到低).
 * 第一个成功 fetch 的 slug 会被采用.
 */
function candidateSlugs(name: string, path?: string): string[] {
    const out: string[] = []
    const seen = new Set<string>()

    function push(s: string) {
        const slug = toLobehubSlug(s)
        if (slug && !seen.has(slug)) {
            seen.add(slug)
            out.push(slug)
        }
    }

    // 1) 完整 name
    push(name)
    // 2) 拆词: 拿每个非空 token 单独试 (e.g., "Microsoft Word" -> "microsoft", "word")
    for (const tok of name.split(/[\s·•]+/)) push(tok)
    // 3) 文件名 (无扩展名)
    if (path) {
        const m = path.match(/([^\\/]+?)(?:\.[a-z0-9]+)?$/i)
        if (m) push(m[1])
    }
    return out
}

/** 单次 fetch + 解析 (带超时, 永不抛错). */
async function tryFetch(slug: string): Promise<string | null> {
    const url = `https://lobehub.com/icon-colorful/${slug}`
    const ctrl = new AbortController()
    const timer = setTimeout(() => ctrl.abort(), LOBEHUB_TIMEOUT_MS)
    try {
        const r = await fetch(url, { signal: ctrl.signal, mode: 'cors' })
        if (!r.ok) return null
        const ct = r.headers.get('content-type') || ''
        // 命中但返回了 HTML/JSON (lobehub 404 页) -> 视为失败
        if (!ct.includes('svg') && !ct.includes('xml') && !ct.includes('octet-stream')) {
            return null
        }
        const text = await r.text()
        if (!text.includes('<svg') || text.length < 50) return null
        // 防止 lobehub 返回了它的 HTML 错误页 (有时 CT 缺失)
        if (text.includes('<!DOCTYPE html>') || text.includes('<html')) return null
        // 转 data URL, 浏览器可直接 <img src> 渲染
        return `data:image/svg+xml;utf8,${encodeURIComponent(text)}`
    } catch {
        return null
    } finally {
        clearTimeout(timer)
    }
}

/**
 * 主入口: 模糊匹配 lobehub 彩色图标.
 *
 * @returns `{ kind: 'svg', value: dataURL }` on hit, `null` on miss/error.
 * 调用方应仅在 Tauri 环境调用 (浏览器 mock 模式下, 我们故意不发起请求).
 */
export async function lobehubFuzzyMatch(
    name: string,
    path?: string,
): Promise<IconState | null> {
    if (!name && !path) return null
    const slugs = candidateSlugs(name || '', path)
    for (const s of slugs) {
        const dataUrl = await tryFetch(s)
        if (dataUrl) {
            return { kind: 'svg', value: dataUrl }
        }
    }
    return null
}
