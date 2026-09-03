import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react'
import { Link } from 'react-router'
import { Logo } from '../logo'
import {
  COMPONENT_NAV_GROUPS,
  SWIFT_UI_NAV_GROUP,
  componentDocsPath,
  type ComponentDoc,
} from '../../lib/component-docs'
import {
  DOCS_PAGES,
  docsPath,
  type DocsOutlineItem,
  type DocsSlug,
} from '../../lib/docs'
import {
  LOCALE_LABELS,
  SUPPORTED_LOCALES,
  localePath,
  type Locale,
} from '../../i18n'
import { site } from '../../lib/site'

type DocsTheme = 'light' | 'dark'
export type DocsArea = 'guide' | 'components' | 'swift-ui'

export interface DocsShellPage {
  title: string
  description: string
  outline: readonly DocsOutlineItem[]
  path: string
  area: DocsArea
}

interface NavItem {
  title: string
  description: string
  path: string
  terms: string
}

interface NavGroup {
  title: string
  items: readonly NavItem[]
}

const sidebarScrollTop = {
  desktop: 0,
  mobile: 0,
}

const ui = {
  en: {
    guide: 'Guide',
    components: 'Components',
    swiftUi: 'SwiftUI',
    introduction: 'Introduction',
    solid: 'Solid',
    primitives: 'Primitives',
    forms: 'Forms & Controls',
    layout: 'Layout & Data',
    overlays: 'Overlays',
    menus: 'Menus & Navigation',
    swiftComponents: 'SwiftUI Components',
    search: 'Search',
    searchDocs: 'Search documentation',
    noResults: 'No documentation found.',
    onThisPage: 'On this page',
    menu: 'Menu',
    previous: 'Previous page',
    next: 'Next page',
    skip: 'Skip to content',
    language: 'Language',
    theme: 'Switch to {{theme}} theme',
    light: 'light',
    dark: 'dark',
  },
  zh: {
    guide: '指南',
    components: '组件',
    swiftUi: 'SwiftUI',
    introduction: '简介',
    solid: 'Solid',
    primitives: '基础组件',
    forms: '表单与控件',
    layout: '布局与数据',
    overlays: '浮层',
    menus: '菜单与导航',
    swiftComponents: 'SwiftUI 组件',
    search: '搜索',
    searchDocs: '搜索文档',
    noResults: '未找到相关文档。',
    onThisPage: '本页内容',
    menu: '菜单',
    previous: '上一页',
    next: '下一页',
    skip: '跳到正文',
    language: '语言',
    theme: '切换到{{theme}}主题',
    light: '浅色',
    dark: '深色',
  },
  ja: {
    guide: 'ガイド',
    components: 'コンポーネント',
    swiftUi: 'SwiftUI',
    introduction: 'はじめに',
    solid: 'Solid',
    primitives: 'プリミティブ',
    forms: 'フォームとコントロール',
    layout: 'レイアウトとデータ',
    overlays: 'オーバーレイ',
    menus: 'メニューとナビゲーション',
    swiftComponents: 'SwiftUI コンポーネント',
    search: '検索',
    searchDocs: 'ドキュメントを検索',
    noResults: '該当するドキュメントはありません。',
    onThisPage: 'このページの内容',
    menu: 'メニュー',
    previous: '前のページ',
    next: '次のページ',
    skip: '本文へスキップ',
    language: '言語',
    theme: '{{theme}}テーマに切り替え',
    light: 'ライト',
    dark: 'ダーク',
  },
} as const

function localize(locale: Locale, path: string): string {
  return locale === 'en' ? path : `/${locale}${path}`
}

function guideItem(slug: DocsSlug): NavItem {
  const page = DOCS_PAGES.find((candidate) => candidate.slug === slug)
  if (!page) throw new Error(`Unknown docs page: ${slug}`)
  return {
    title: page.title,
    description: page.description,
    path: docsPath(page.slug),
    terms: page.searchTerms.join(' '),
  }
}

function componentItem(component: ComponentDoc): NavItem {
  return {
    title: component.name,
    description: component.description,
    path: componentDocsPath(component),
    terms: `${component.section} ${component.parts.join(' ')} ${component.keyProps.join(' ')}`,
  }
}

function navGroups(locale: Locale): readonly NavGroup[] {
  const labels = ui[locale]
  const componentGroupTitles: Record<string, string> = {
    Primitives: labels.primitives,
    'Forms & Controls': labels.forms,
    'Layout & Data': labels.layout,
    Overlays: labels.overlays,
    'Menus & Navigation': labels.menus,
  }

  return [
    {
      title: labels.introduction,
      items: [guideItem('getting-started'), guideItem('project-structure')],
    },
    {
      title: labels.solid,
      items: [guideItem('solid'), guideItem('styling')],
    },
    {
      title: labels.components,
      items: [
        guideItem('components'),
        guideItem('forms-and-input'),
        guideItem('overlays-and-dialogs'),
      ],
    },
    ...COMPONENT_NAV_GROUPS.map((group) => ({
      title: componentGroupTitles[group.title] ?? group.title,
      items: group.items.map(componentItem),
    })),
    {
      title: labels.swiftUi,
      items: [guideItem('swift-ui'), guideItem('swift-ui-hosting')],
    },
    {
      title: labels.swiftComponents,
      items: SWIFT_UI_NAV_GROUP.items.map(componentItem),
    },
  ]
}

function DocsSidebar({
  currentPath,
  locale,
  mobile = false,
  onNavigate,
}: {
  currentPath: string
  locale: Locale
  mobile?: boolean
  onNavigate?: () => void
}) {
  const sidebarRef = useRef<HTMLElement>(null)
  const scrollKey = mobile ? 'mobile' : 'desktop'

  useLayoutEffect(() => {
    if (sidebarRef.current) {
      sidebarRef.current.scrollTop = sidebarScrollTop[scrollKey]
    }
  }, [scrollKey])

  function rememberScrollPosition() {
    if (sidebarRef.current) {
      sidebarScrollTop[scrollKey] = sidebarRef.current.scrollTop
    }
  }

  return (
    <aside
      ref={sidebarRef}
      className={mobile ? 'docs-mobile-drawer' : 'docs-sidebar'}
      aria-label={mobile ? ui[locale].menu : undefined}
      onScroll={rememberScrollPosition}
    >
      <nav aria-label={ui[locale].menu}>
        {navGroups(locale).map((group) => (
          <section className="docs-nav-group" key={group.title}>
            <h2>{group.title}</h2>
            <ul>
              {group.items.map((item) => (
                <li key={item.path}>
                  <Link
                    to={localize(locale, item.path)}
                    aria-current={currentPath === item.path ? 'page' : undefined}
                    onClick={() => {
                      rememberScrollPosition()
                      onNavigate?.()
                    }}
                  >
                    {item.title}
                  </Link>
                </li>
              ))}
            </ul>
          </section>
        ))}
      </nav>
    </aside>
  )
}

function DocsOutline({
  page,
  locale,
  mobile = false,
  onNavigate,
}: {
  page: DocsShellPage
  locale: Locale
  mobile?: boolean
  onNavigate?: () => void
}) {
  const content = (
    <nav aria-label={ui[locale].onThisPage}>
      <h2>
        <span className="i-lucide-list-filter" aria-hidden />
        {ui[locale].onThisPage}
      </h2>
      <ul>
        {page.outline.map((item) => (
          <li key={item.id} data-level={item.level ?? 2}>
            <a href={`#${item.id}`} onClick={onNavigate}>
              {item.title}
            </a>
          </li>
        ))}
      </ul>
    </nav>
  )

  return mobile ? (
    <aside className="docs-mobile-drawer docs-mobile-outline">{content}</aside>
  ) : (
    <aside className="docs-outline">{content}</aside>
  )
}

function SearchDialog({
  open,
  locale,
  onClose,
}: {
  open: boolean
  locale: Locale
  onClose: () => void
}) {
  const [query, setQuery] = useState('')
  const inputRef = useRef<HTMLInputElement>(null)
  const groups = useMemo(() => navGroups(locale), [locale])
  const pages = useMemo(() => groups.flatMap((group) => group.items), [groups])

  useEffect(() => {
    if (!open) return
    setQuery('')
    requestAnimationFrame(() => inputRef.current?.focus())
  }, [open])

  const results = useMemo(() => {
    const needle = query.trim().toLocaleLowerCase()
    if (!needle) return pages
    return pages.filter((page) =>
      `${page.title} ${page.description} ${page.terms}`
        .toLocaleLowerCase()
        .includes(needle),
    )
  }, [pages, query])

  if (!open) return null

  return (
    <div className="docs-search-backdrop" onMouseDown={onClose}>
      <section
        className="docs-search-dialog"
        role="dialog"
        aria-modal="true"
        aria-label={ui[locale].searchDocs}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <div className="docs-search-field">
          <span className="i-lucide-search" aria-hidden />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={ui[locale].searchDocs}
            aria-label={ui[locale].searchDocs}
          />
          <button type="button" onClick={onClose}>Esc</button>
        </div>
        <div className="docs-search-results">
          {results.length ? (
            results.map((result) => (
              <a href={localize(locale, result.path)} key={result.path}>
                <span>{result.title}</span>
                <small>{result.description}</small>
              </a>
            ))
          ) : (
            <p>{ui[locale].noResults}</p>
          )}
        </div>
      </section>
    </div>
  )
}

function DocsHeader({
  area,
  currentPath,
  locale,
  theme,
  menuOpen,
  onMenuToggle,
  onSearch,
  onThemeToggle,
}: {
  area: DocsArea
  currentPath: string
  locale: Locale
  theme: DocsTheme
  menuOpen: boolean
  onMenuToggle: () => void
  onSearch: () => void
  onThemeToggle: () => void
}) {
  const labels = ui[locale]
  const headerLinks = [
    { label: labels.guide, href: '/docs', area: 'guide' },
    { label: labels.components, href: '/docs/components', area: 'components' },
    { label: labels.swiftUi, href: '/docs/swift-ui', area: 'swift-ui' },
  ] as const

  return (
    <header className="docs-header">
      <div className="docs-frame docs-header-inner">
        <a href={localePath(locale)} className="docs-brand" aria-label="QuickGUI home">
          <Logo />
          <span>{site.name}</span>
        </a>

        <nav className="docs-header-nav" aria-label="Documentation">
          {headerLinks.map((link) => (
            <a
              key={link.area}
              href={localize(locale, link.href)}
              aria-current={link.area === area ? 'page' : undefined}
            >
              {link.label}
            </a>
          ))}
        </nav>

        <div className="docs-header-actions">
          <button
            type="button"
            className="docs-search-button"
            onClick={onSearch}
            aria-label={labels.searchDocs}
          >
            <span className="i-lucide-search" aria-hidden />
            <span>{labels.search}</span>
            <kbd>⌘ K</kbd>
          </button>
          <label className="docs-language-select">
            <span className="sr-only">{labels.language}</span>
            <select
              value={locale}
              aria-label={labels.language}
              onChange={(event) => {
                window.location.assign(
                  localize(event.currentTarget.value as Locale, currentPath),
                )
              }}
            >
              {SUPPORTED_LOCALES.map((candidate) => (
                <option key={candidate} value={candidate}>
                  {LOCALE_LABELS[candidate]}
                </option>
              ))}
            </select>
          </label>
          <button
            type="button"
            className="docs-theme-button"
            onClick={onThemeToggle}
            aria-label={labels.theme.replace(
              '{{theme}}',
              theme === 'light' ? labels.dark : labels.light,
            )}
          >
            <span
              className={theme === 'light' ? 'i-lucide-sun' : 'i-lucide-moon'}
              aria-hidden
            />
          </button>
          <a
            className="docs-github-link"
            href={site.links.github}
            target="_blank"
            rel="noreferrer"
            aria-label="QuickGUI on GitHub"
          >
            <span className="i-simple-icons-github" aria-hidden />
          </a>
          <button
            type="button"
            className="docs-mobile-menu-button"
            onClick={onMenuToggle}
            aria-expanded={menuOpen}
            aria-label={labels.menu}
          >
            <span className={menuOpen ? 'i-lucide-x' : 'i-lucide-menu'} aria-hidden />
          </button>
        </div>
      </div>
    </header>
  )
}

function DocsPager({ page, locale }: { page: DocsShellPage; locale: Locale }) {
  const pages = navGroups(locale).flatMap((group) => group.items)
  const index = pages.findIndex((candidate) => candidate.path === page.path)
  const previous = index > 0 ? pages[index - 1] : undefined
  const next = index >= 0 && index < pages.length - 1 ? pages[index + 1] : undefined

  return (
    <nav className="docs-pager" aria-label="Documentation pages">
      {previous ? (
        <a href={localize(locale, previous.path)} className="docs-pager-previous">
          <small>{ui[locale].previous}</small>
          <span>{previous.title}</span>
        </a>
      ) : <span />}
      {next ? (
        <a href={localize(locale, next.path)} className="docs-pager-next">
          <small>{ui[locale].next}</small>
          <span>{next.title}</span>
        </a>
      ) : null}
    </nav>
  )
}

export function DocsShell({
  page,
  locale,
  children,
}: {
  page: DocsShellPage
  locale: Locale
  children: ReactNode
}) {
  const [theme, setTheme] = useState<DocsTheme>('light')
  const [mobilePanel, setMobilePanel] = useState<'menu' | 'outline' | null>(null)
  const [searchOpen, setSearchOpen] = useState(false)

  useEffect(() => {
    const saved = window.localStorage.getItem('quickgui-docs-theme')
    if (saved === 'light' || saved === 'dark') setTheme(saved)
  }, [])

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault()
        setSearchOpen(true)
      } else if (event.key === 'Escape') {
        setSearchOpen(false)
        setMobilePanel(null)
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])

  function toggleTheme() {
    setTheme((current) => {
      const next = current === 'light' ? 'dark' : 'light'
      window.localStorage.setItem('quickgui-docs-theme', next)
      return next
    })
  }

  return (
    <div className="docs-root" data-theme={theme}>
      <a className="docs-skip-link" href="#docs-content">{ui[locale].skip}</a>
      <DocsHeader
        area={page.area}
        currentPath={page.path}
        locale={locale}
        theme={theme}
        menuOpen={mobilePanel === 'menu'}
        onMenuToggle={() => setMobilePanel((current) => current === 'menu' ? null : 'menu')}
        onSearch={() => setSearchOpen(true)}
        onThemeToggle={toggleTheme}
      />

      <div className="docs-mobile-local-nav">
        <button
          type="button"
          aria-expanded={mobilePanel === 'menu'}
          onClick={() => setMobilePanel((current) => current === 'menu' ? null : 'menu')}
        >
          <span className="i-lucide-list-filter" aria-hidden />
          {ui[locale].menu}
        </button>
        <button
          type="button"
          aria-expanded={mobilePanel === 'outline'}
          onClick={() => setMobilePanel((current) => current === 'outline' ? null : 'outline')}
        >
          {ui[locale].onThisPage}
          <span
            className={mobilePanel === 'outline' ? 'i-lucide-chevron-up' : 'i-lucide-chevron-right'}
            aria-hidden
          />
        </button>
      </div>

      <div className="docs-frame docs-layout">
        <DocsSidebar currentPath={page.path} locale={locale} />
        <div className="docs-content-column">
          <main id="docs-content" className="docs-article">
            <h1>{page.title}</h1>
            {children}
            <DocsPager page={page} locale={locale} />
          </main>
        </div>
        <DocsOutline page={page} locale={locale} />
      </div>

      {mobilePanel ? (
        <div className="docs-mobile-panel-backdrop" onMouseDown={() => setMobilePanel(null)}>
          <div onMouseDown={(event) => event.stopPropagation()}>
            {mobilePanel === 'menu' ? (
              <DocsSidebar
                currentPath={page.path}
                locale={locale}
                mobile
                onNavigate={() => setMobilePanel(null)}
              />
            ) : (
              <DocsOutline
                page={page}
                locale={locale}
                mobile
                onNavigate={() => setMobilePanel(null)}
              />
            )}
          </div>
        </div>
      ) : null}

      <SearchDialog open={searchOpen} locale={locale} onClose={() => setSearchOpen(false)} />
    </div>
  )
}
