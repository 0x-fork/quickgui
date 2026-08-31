export function loader() {
  throw new Response('Page not found', { status: 404 })
}

export default function NotFound() {
  return null
}
