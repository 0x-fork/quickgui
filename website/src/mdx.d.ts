declare module '*.mdx' {
  import type { ComponentType } from 'react'

  const MDXContent: ComponentType<{
    components?: Readonly<Record<string, ComponentType<any>>>
  }>

  export default MDXContent
}
