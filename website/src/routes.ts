import { route, type RouteConfig } from '@react-router/dev/routes'

export default [
  route('og', 'routes/og.tsx'),
  route(':locale?', 'routes/home.tsx'),
  route('*', 'routes/not-found.tsx'),
] satisfies RouteConfig
