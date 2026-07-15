/**
 * 文件后缀 → 子分类 映射表.
 *
 * 命名规则遵循 Raycast / Everything 等主流文件搜索工具, 由常用扩展名按"用途"汇总:
 *   - document: Office / 文档 / 文本 / 阅读
 *   - image:    摄影 / 矢量 / 设计
 *   - video:    视频剪辑 / 录像
 *   - audio:    音乐 / 录音
 *   - executable:编程 / 脚本 / 安装包
 *   - archive:  压缩 / 镜像
 *   - code:     源代码(我们**额外**分出一类, 不归上面任何类)
 *   - pdf:      PDF (单列, 用户常用)
 *   - font:     字体
 *   - design:   设计/原型
 *
 * 不在表内 → 落到 VirtualGroupedResults 的 "所有文件" 默认分组(或 "其他文件").
 */
export type FileKind =
  | 'document'
  | 'image'
  | 'video'
  | 'audio'
  | 'executable'
  | 'archive'
  | 'code'
  | 'pdf'
  | 'font'
  | 'design'
  | 'spreadsheet'
  | 'presentation'
  | 'other'

const DOC_EXT = new Set([
  'txt','md','markdown','rtf','log','rst','doc','docx','odt','pages',
  'epub','mobi','azw','azw3','fb2','djvu','tex',
])

const IMAGE_EXT = new Set([
  'jpg','jpeg','png','gif','webp','bmp','tiff','tif','heic','heif',
  'ico','avif','jxl','raw','cr2','nef','orf','sr2','dng','arw','rw2','raf',
])

const VIDEO_EXT = new Set([
  'mp4','m4v','mkv','webm','mov','avi','wmv','flv','f4v','mpeg','mpg',
  'mp2','mpe','vob','ogv','3gp','3g2','mts','m2ts','ts','rm','rmvb',
])

const AUDIO_EXT = new Set([
  'mp3','m4a','aac','flac','wav','ogg','oga','opus','wma','alac','ape',
  'aiff','aif','aifc','mka','caf','mid','midi','amr','awb',
])

const EXECUTABLE_EXT = new Set([
  'exe','msi','bat','cmd','com','scr','pif','gadget','dll','sys','drv',
  'bin','run','app','dmg','pkg','deb','rpm','apk','ipa','appx','msix',
  'jar','jse','wsf','vbs','ps1','psm1','sh','bash','zsh','ksh','csh',
])

const ARCHIVE_EXT = new Set([
  'zip','rar','7z','tar','gz','tgz','bz2','tbz','xz','txz','lz','lzma',
  'lz4','zst','Z','cab','msp','msu','rpm','iso','img','dmg','wim','esd',
])

const CODE_EXT = new Set([
  'js','mjs','cjs','ts','tsx','vue','jsx','html','htm','css','scss','sass',
  'less','styl','json','jsonc','json5','yaml','yml','toml','xml','ini','conf',
  'cfg','properties','env','gitignore','gitattributes','dockerignore',
  'editorconfig','eslintignore','prettierrc','lock','lockb','sum','sig',
  'rs','go','py','pyi','java','kt','kts','scala','rb','erb','php','phtml',
  'c','h','cpp','cc','cxx','hpp','hxx','m','mm','swift','d','md','sql',
  'graphql','gql','proto','thrift','rs','dart','lua','pl','pm','t','r','jl',
  'f','f90','f95','for','asm','s','S','clj','cljs','cljc','edn','ex','exs',
  'elm','erl','hrl','fs','fsx','fsproj','purs','re','rei','sml','sig',
])

const PDF_EXT = new Set(['pdf'])

const FONT_EXT = new Set([
  'ttf','otf','woff','woff2','eot','fon','dfont','pfb','pfm','sfnt',
])

const DESIGN_EXT = new Set([
  'psd','ai','sketch','fig','xd','afdesign','afphoto','afpub','indd',
  'svg','eps','emf','wmf','cdr','dxf','dwg','uxt','uipen','uidesigner','xmind','pma','pmd',
])

const SPREADSHEET_EXT = new Set([
  'xls','xlsx','xlsm','xlsb','csv','tsv','ods','numbers','gsheet','fods',
])

const PRESENTATION_EXT = new Set([
  'ppt','pptx','pps','ppsx','odp','key','fodp',
])

/** 根据扩展名返回文件细分类. 不区分大小写, 无扩展或未识别 → 'other'. */
export function classify(extOrName: string): FileKind {
  if (!extOrName) return 'other'
  // 接受 "name.ext" 或直接 ".ext"
  const dot = extOrName.lastIndexOf('.')
  const raw = dot >= 0 ? extOrName.slice(dot + 1) : extOrName
  const e = raw.toLowerCase()
  if (!e) return 'other'

  if (PDF_EXT.has(e)) return 'pdf'
  if (DOC_EXT.has(e)) return 'document'
  if (SPREADSHEET_EXT.has(e)) return 'spreadsheet'
  if (PRESENTATION_EXT.has(e)) return 'presentation'
  if (IMAGE_EXT.has(e)) return 'image'
  if (VIDEO_EXT.has(e)) return 'video'
  if (AUDIO_EXT.has(e)) return 'audio'
  if (EXECUTABLE_EXT.has(e)) return 'executable'
  if (ARCHIVE_EXT.has(e)) return 'archive'
  if (CODE_EXT.has(e)) return 'code'
  if (FONT_EXT.has(e)) return 'font'
  if (DESIGN_EXT.has(e)) return 'design'
  return 'other'
}

/**
 * 根据后端返回的 resultType 推断文件分类 (用于无扩展名 / 目录等场景)。
 * 优先于 classify() 提供的扩展名映射, 因为后端的分类更准 (基于 resultType 字段).
 */
export function classifyByResultType(resultType: string | undefined): FileKind | null {
  if (!resultType) return null
  switch (resultType) {
    case 'directory': return 'other'  // 目录归到 other (但一般目录不参与"所有文件"分组, 而是单独"所有目录"分组)
    case 'document':
    case 'spreadsheet':
    case 'presentation':
    case 'pdf':
    case 'code':
    case 'image':
    case 'video':
    case 'audio':
    case 'executable':
    case 'archive':
    case 'other-file':
      return resultType as FileKind
    default:
      return null
  }
}

/**
 * 从 SearchResult 推导 FileKind, 统一入口.
 *
 * 优先级:
 * 1. resultType (后端分类更准, 包括目录等无扩展名场景)
 * 2. subtitle/title 文件名后缀 (前端兜底)
 *
 * 不在表内 → 'other'.
 *
 * 抽到此处作为"分类表单一真源": stores/search.ts 与任何 UI 组件
 * 都通过此函数获取 FileKind, 避免各组件重复定义扩展名集合.
 */
export function getFileKind(r: { resultType?: string; title?: string; subtitle?: string }): FileKind {
  if (r.resultType) {
    const k = classifyByResultType(r.resultType)
    if (k) return k
  }
  // 回退: 从扩展名推断
  const path = r.subtitle || r.title || ''
  const name = path.split(/[\\/]/).pop() || ''
  return classify(name)
}

export interface FileKindMeta {
  id: FileKind
  label: string
  iconKey: 'document' | 'image' | 'video' | 'audio' | 'code' | 'pdf'
         | 'archive' | 'font' | 'design' | 'spreadsheet' | 'presentation'
         | 'executable' | 'other-file'
}

export const FILE_KIND_META: Record<FileKind, FileKindMeta> = {
  document:      { id: 'document',      label: '文档',   iconKey: 'document' },
  spreadsheet:   { id: 'spreadsheet',   label: '表格',   iconKey: 'spreadsheet' },
  presentation:  { id: 'presentation',  label: '演示',   iconKey: 'presentation' },
  pdf:           { id: 'pdf',           label: 'PDF',    iconKey: 'pdf' },
  code:          { id: 'code',          label: '代码',   iconKey: 'code' },
  image:         { id: 'image',         label: '图片',   iconKey: 'image' },
  video:         { id: 'video',         label: '视频',   iconKey: 'video' },
  audio:         { id: 'audio',         label: '音频',   iconKey: 'audio' },
  executable:    { id: 'executable',    label: '程序',   iconKey: 'executable' },
  archive:       { id: 'archive',       label: '压缩包', iconKey: 'archive' },
  font:          { id: 'font',          label: '字体',   iconKey: 'font' },
  design:        { id: 'design',        label: '设计',   iconKey: 'design' },
  other:         { id: 'other',         label: '其他',   iconKey: 'other-file' },
}

/** 用于前端的"可见"分类顺序 (在所有文件分组标题下方展示为多选 chip). */
export const FILE_KIND_DISPLAY_ORDER: FileKind[] = [
  'document', 'spreadsheet', 'presentation', 'pdf', 'code',
  'image', 'video', 'audio', 'archive', 'font', 'design', 'executable', 'other',
]
