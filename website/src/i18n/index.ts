import { createInstance, type i18n as I18nInstance } from 'i18next'
import { initReactI18next } from 'react-i18next'
import { en } from './messages/en'
import { zh } from './messages/zh'
import { ja } from './messages/ja'

export const SUPPORTED_LOCALES = ['en', 'zh', 'ja'] as const
export type Locale = (typeof SUPPORTED_LOCALES)[number]

export const messages = { en, zh, ja } as const

export const LOCALE_LABELS: Record<Locale, string> = {
  en: 'EN',
  zh: '中文',
  ja: '日本語',
}

export const OG_LOCALES: Record<Locale, string> = {
  en: 'en_US',
  zh: 'zh_CN',
  ja: 'ja_JP',
}

/** `undefined` (no prefix) is English; unknown prefixes resolve to null → 404. */
export function resolveLocale(param: string | undefined): Locale | null {
  if (param === undefined) return 'en'
  return param === 'zh' || param === 'ja' ? param : null
}

export function localePath(locale: Locale): string {
  return locale === 'en' ? '/' : `/${locale}`
}

export function htmlLang(locale: Locale): string {
  return locale === 'zh' ? 'zh-CN' : locale
}

/**
 * One instance per rendered tree (created in component state), so concurrent
 * SSR requests on the Worker can never see each other's language.
 */
export function createI18n(locale: Locale): I18nInstance {
  const instance = createInstance()
  instance.use(initReactI18next)
  void instance.init({
    resources: {
      en: { translation: en },
      zh: { translation: zh },
      ja: { translation: ja },
    },
    lng: locale,
    fallbackLng: 'en',
    interpolation: { escapeValue: false },
    initAsync: false,
    react: { useSuspense: false },
  })
  return instance
}
