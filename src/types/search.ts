export type SearchCategory = 'apps' | 'files' | 'commands'

export type ResultType =
  | 'system-app'
  | 'user-app'
  | 'uwp-app'
  | 'directory'
  | 'document'
  | 'image'
  | 'video'
  | 'audio'
  | 'executable'
  | 'archive'
  | 'other-file'
  | 'command'

export interface SearchOptions {
  categories: SearchCategory[]
  maxResults: number
  includeHidden: boolean
}

export type SearchAction =
  | { type: 'launch'; data: string }
  | { type: 'open'; data: string }
  | { type: 'run'; data: { command: string; args: string[] } }
  | { type: 'navigate'; data: string }

export interface SearchResult {
  id: string
  title: string
  /**
   * 主副标题: 常规文件为**绝对路径**; 应用为空字符串; 命令为 "command args".
   */
  subtitle: string
  /**
   * 次级元信息 (右侧灰色小字): 文件为人类可读大小, 其他类型可空.
   * 与 subtitle 解耦, 让"路径"就是路径, "大小"就是大小, 视觉上不混淆.
   */
  meta?: string | null
  icon: string | null
  category: SearchCategory
  resultType: ResultType
  action: SearchAction
  score: number
}
