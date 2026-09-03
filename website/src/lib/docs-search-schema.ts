export const DOCS_SEARCH_SCHEMA = {
  pageTitle: 'string',
  section: 'string',
  hierarchy: 'string',
  content: 'string',
  keywords: 'string',
  url: 'string',
  area: 'string',
} as const

export type DocsSearchArea = 'guide' | 'solid' | 'swift-ui'

export interface DocsSearchDocument {
  id: string
  pageTitle: string
  section: string
  hierarchy: string
  content: string
  keywords: string
  url: string
  area: DocsSearchArea
}
