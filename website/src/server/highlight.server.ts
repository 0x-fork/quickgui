import type { HighlightedSnippets, SnippetKey } from '../lib/snippets'

let cached: Promise<HighlightedSnippets> | null = null

async function highlightAll(): Promise<HighlightedSnippets> {
  const [
    { snippets },
    { createHighlighterCore },
    { createJavaScriptRegexEngine },
    rust,
    tsx,
    bash,
    githubLight,
  ] = await Promise.all([
    import('../lib/snippets'),
    import('shiki/core'),
    import('shiki/engine/javascript'),
    import('shiki/langs/rust.mjs'),
    import('shiki/langs/tsx.mjs'),
    import('shiki/langs/bash.mjs'),
    import('shiki/themes/github-light.mjs'),
  ])

  const highlighter = await createHighlighterCore({
    themes: [githubLight.default],
    langs: [rust.default, tsx.default, bash.default],
    engine: createJavaScriptRegexEngine({ forgiving: true }),
  })

  const out = {} as HighlightedSnippets
  for (const key of Object.keys(snippets) as Array<SnippetKey>) {
    const { lang, code } = snippets[key]
    out[key] = highlighter.codeToHtml(code, { lang, theme: 'github-light' })
  }
  return out
}

export function getHighlightedSnippets(): Promise<HighlightedSnippets> {
  cached ??= highlightAll()
  return cached
}
