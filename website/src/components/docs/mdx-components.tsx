import {
  Children,
  isValidElement,
  useRef,
  useState,
  type ComponentProps,
  type ComponentPropsWithoutRef,
  type ReactNode,
} from 'react'
import type { Locale } from '../../i18n'
import { Note } from './docs-content'

const ui = {
  en: {
    copied: 'Copied',
    copyCode: 'Copy code',
    headingLink: (title: string) => `Link to ${title}`,
    note: 'NOTE',
  },
  zh: {
    copied: '已复制',
    copyCode: '复制代码',
    headingLink: (title: string) => `链接到“${title}”`,
    note: '注意',
  },
  ja: {
    copied: 'コピーしました',
    copyCode: 'コードをコピー',
    headingLink: (title: string) => `${title} へのリンク`,
    note: '注意',
  },
} as const

function localizedHref(locale: Locale, href: string | undefined): string | undefined {
  if (!href?.startsWith('/docs') || locale === 'en') return href
  return `/${locale}${href}`
}

function textContent(node: ReactNode): string {
  if (typeof node === 'string' || typeof node === 'number') return String(node)
  if (Array.isArray(node)) return node.map(textContent).join('')
  if (isValidElement<{ children?: ReactNode }>(node)) {
    return textContent(node.props.children)
  }
  return ''
}

function languageFrom(children: ReactNode): string | undefined {
  const first = Children.toArray(children)[0]
  if (!isValidElement<{ className?: string }>(first)) return undefined
  return first.props.className?.match(/language-([^\s]+)/)?.[1]
}

function MdxPre({
  children,
  locale,
  ...props
}: ComponentPropsWithoutRef<'pre'> & {
  'data-language'?: string
  locale: Locale
}) {
  const [copied, setCopied] = useState(false)
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null)
  const language = props['data-language'] ?? languageFrom(children)

  async function copy() {
    try {
      await navigator.clipboard.writeText(textContent(children).replace(/\n$/, ''))
      setCopied(true)
      if (timer.current) clearTimeout(timer.current)
      timer.current = setTimeout(() => setCopied(false), 1400)
    } catch {
      // Clipboard access can be denied in embedded or insecure previews.
    }
  }

  return (
    <div className="docs-code-block">
      {language ? <span className="docs-code-language">{language}</span> : null}
      <button
        type="button"
        onClick={copy}
        className="docs-code-copy"
        aria-label={copied ? ui[locale].copied : ui[locale].copyCode}
      >
        <span className={copied ? 'i-lucide-check' : 'i-lucide-copy'} aria-hidden />
      </button>
      <div className="docs-code-source">
        <pre {...props} tabIndex={0}>{children}</pre>
      </div>
    </div>
  )
}

function MdxHeading({
  level,
  id,
  children,
  locale,
  ...props
}: (ComponentPropsWithoutRef<'h2'> | ComponentPropsWithoutRef<'h3'>) & {
  level: 2 | 3
  locale: Locale
}) {
  const content = (
    <>
      {id ? (
        <a
          className="docs-heading-anchor"
          href={`#${id}`}
          aria-label={ui[locale].headingLink(textContent(children))}
        >
          #
        </a>
      ) : null}
      {children}
    </>
  )

  return level === 2 ? (
    <h2 id={id} {...props}>{content}</h2>
  ) : (
    <h3 id={id} {...props}>{content}</h3>
  )
}

function MdxTable({
  children,
  ...props
}: ComponentPropsWithoutRef<'table'>) {
  return (
    <div className="docs-table-wrap">
      <table {...props}>{children}</table>
    </div>
  )
}

export function getDocsMdxComponents(locale: Locale) {
  return {
    pre: (props: ComponentPropsWithoutRef<'pre'>) => (
      <MdxPre locale={locale} {...props} />
    ),
    table: MdxTable,
    h2: (props: ComponentPropsWithoutRef<'h2'>) => (
      <MdxHeading level={2} locale={locale} {...props} />
    ),
    h3: (props: ComponentPropsWithoutRef<'h3'>) => (
      <MdxHeading level={3} locale={locale} {...props} />
    ),
    a: ({ href, ...props }: ComponentPropsWithoutRef<'a'>) => (
      <a href={localizedHref(locale, href)} {...props} />
    ),
    Note: ({ title, ...props }: ComponentProps<typeof Note>) => (
      <Note title={title ?? ui[locale].note} {...props} />
    ),
  }
}
