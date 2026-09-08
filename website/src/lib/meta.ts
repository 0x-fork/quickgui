import type { MetaDescriptor } from 'react-router'
import { site } from './site'

export function siteMeta(
  origin: string,
  title: string = site.name,
): MetaDescriptor[] {
  return [
    { title },
    { name: 'theme-color', content: '#ffffff', media: '(prefers-color-scheme: light)' },
    { name: 'theme-color', content: '#171719', media: '(prefers-color-scheme: dark)' },
    { property: 'og:site_name', content: site.name },
    { property: 'og:type', content: 'website' },
    { property: 'og:image', content: `${origin}/og.png` },
    { name: 'twitter:card', content: 'summary_large_image' },
    { name: 'twitter:image', content: `${origin}/og.png` },
  ]
}
