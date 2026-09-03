import { route, type RouteConfig } from '@react-router/dev/routes'

export default [
  route('og', 'routes/og.tsx'),
  route(':locale?/docs/:family/:component', 'routes/docs-component.tsx'),
  route(':locale?/docs/:slug?', 'routes/docs.tsx'),
  route(':locale?', 'routes/home.tsx'),
  route('*', 'routes/not-found.tsx'),
] satisfies RouteConfig
