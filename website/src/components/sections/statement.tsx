import { useTranslation } from 'react-i18next'
import { Container } from '../container'
import { site } from '../../lib/site'

export function Statement() {
  const { t } = useTranslation()

  return (
    <section>
      <Container>
        <div className="rounded-3xl bg-cream px-8 py-20 text-center text-cream-foreground sm:px-16 sm:py-28">
          <p className="mx-auto max-w-3xl text-3xl font-semibold tracking-tight text-balance sm:text-4xl">
            {t('statement.text')}
          </p>
          <a
            href={site.links.architecture}
            target="_blank"
            rel="noreferrer"
            className="mt-10 inline-flex h-11 items-center gap-2 rounded-md bg-cream-foreground px-5 text-[15px] font-medium text-cream transition-opacity hover:opacity-85"
          >
            {t('statement.link')}
            <span className="i-lucide-arrow-up-right size-4" aria-hidden />
          </a>
        </div>
      </Container>
    </section>
  )
}
