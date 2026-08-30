import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Container } from '../container'
import { Logo } from '../logo'
import { site } from '../../lib/site'

export function FinalCta() {
  const { t } = useTranslation()

  return (
    <section>
      <Container>
        <div className="relative overflow-hidden rounded-3xl border border-border/60 bg-card px-8 py-24 text-center sm:py-28">
          <div
            aria-hidden
            className="bg-dots absolute inset-0 [mask-image:radial-gradient(ellipse_70%_80%_at_50%_100%,black,transparent)]"
          />
          <div
            aria-hidden
            className="absolute -bottom-52 left-1/2 h-[400px] w-[720px] -translate-x-1/2 rounded-full bg-peach/[0.07] blur-[120px]"
          />
          <div className="relative flex flex-col items-center">
            <Logo className="size-12" />
            <h2 className="mt-8 max-w-xl bg-gradient-to-b from-white to-white/60 bg-clip-text text-4xl font-semibold tracking-tighter text-transparent text-balance sm:text-5xl">
              {t('cta.title')}
            </h2>
            <p className="mt-5 max-w-md text-base leading-relaxed text-muted-foreground">
              {t('cta.body')}
            </p>
            <div className="mt-9 flex flex-wrap items-center justify-center gap-3">
              <Button asChild size="lg" className="h-11 gap-2 px-5 text-[15px]">
                <a href={site.links.docs} target="_blank" rel="noreferrer">
                  {t('cta.docs')}
                  <span
                    className="i-lucide-arrow-up-right size-4"
                    aria-hidden
                  />
                </a>
              </Button>
              <Button
                asChild
                size="lg"
                variant="outline"
                className="h-11 gap-2 px-5 text-[15px]"
              >
                <a href={site.links.github} target="_blank" rel="noreferrer">
                  <span className="i-simple-icons-github size-4" aria-hidden />
                  {t('cta.star')}
                </a>
              </Button>
            </div>
          </div>
        </div>
      </Container>
    </section>
  )
}
