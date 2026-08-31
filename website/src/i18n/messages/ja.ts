import type { en } from './en'

export const ja: typeof en = {
  meta: {
    title: 'QuickGUI — ネイティブデスクトップアプリを作る。WebView はいらない',
    description:
      'QuickGUI は Rust または TypeScript でネイティブデスクトップアプリを作る GPU アクセラレーション GUI フレームワーク。見慣れた Flexbox / Grid レイアウト、ちゃんと使えるテキスト、標準のアクセシビリティ——アイドル時は CPU ゼロ',
  },
  common: {
    skipToContent: 'コンテンツへスキップ',
    getStarted: 'はじめる',
    copy: '“{{text}}” をコピー',
    copied: 'コピーしました',
    language: '言語',
  },
  nav: {
    features: '機能',
    code: 'コード',
    quickstart: 'クイックスタート',
    docs: 'ドキュメント',
  },
  hero: {
    badge: 'v{{version}} crates.io で公開中',
    titleLine1: 'ネイティブデスクトップアプリを作る',
    titleLine2: 'WebView はいらない',
    sub: 'Rust または TypeScript でネイティブアプリを作る GPU アクセラレーション GUI フレームワーク。見慣れたレイアウト、ちゃんと使えるテキスト、標準のアクセシビリティ——アプリがアイドルなら CPU はゼロ',
  },
  features: {
    title: 'デスクトップアプリに必要なもの、ぜんぶ',
    items: {
      idle: {
        title: 'アイドル時は CPU ゼロ',
        body: '画面に変化がなければ、何も動きません。アイドルウィンドウはフレームを描かず、バッテリーも消費しない',
      },
      fast: {
        title: '最初から速い',
        body: '描画は GPU 上で行われ、ウィンドウの変わった部分だけが再描画されます',
      },
      layout: {
        title: '見慣れたレイアウト',
        body: 'Flexbox と CSS Grid、Tailwind 風のショートハンド付き。Web ページを組めるなら、ウィンドウも組めます',
      },
      components: {
        title: 'コンポーネント同梱',
        body: 'アクセシブルでスタイルなしのコンポーネントを多数同梱——メニュー、ダイアログ、ポップオーバー、select、combobox、タブ、テーブル、ツリー。自由に仕上げられます',
      },
      text: {
        title: 'テキストがちゃんと使える',
        body: '選択、編集、アンドゥ、IME、絵文字、右から左へ書く言語——どのネイティブアプリとも同じ挙動',
      },
      a11y: {
        title: '標準でアクセシブル',
        body: 'スクリーンリーダーには本物のボタン・リスト・テキストが見えます。追加コードは不要',
      },
      lists: {
        title: 'どんなサイズでも滑らか',
        body: '100 万行のテーブルやリストも、フレーム落ちなくスクロール',
      },
      native: {
        title: '本物のネイティブ',
        body: '本物のウィンドウ、ネイティブメニュー、ダイアログ、トレイアイコン、通知——ブラウザの着ぐるみではありません',
      },
      cli: {
        title: 'CLI ひとつで開発から配布まで',
        body: 'quickgui dev は編集しながら実行、quickgui build は署名済みでインストール可能なリリースを出力します',
      },
    },
  },
  code: {
    title: 'Rust でも、TypeScript でも',
    lead: '好きな方でどうぞ——どちらも同じネイティブアプリになります。webview はありません',
  },
  swiftUi: {
    title: 'QuickGUI でネイティブ SwiftUI を使う',
    lead: 'macOS の Solid アプリに、本物の SwiftUI コントロールをそのまま埋め込めます。',
  },
  quickstart: {
    title: '1 分ではじめる',
    rust: {
      addCrate: 'クレートを追加',
      write: 'ビューを書く',
      writeBody: '上のカウンターがそのまま完全な main.rs です——コピーするだけ',
      run: '実行',
    },
    solid: {
      create: 'アプリを作成',
      ship: '出荷する',
      note: '本物の署名済み・自己完結アプリが手に入ります。インストーラー付き',
    },
  },
  platforms: {
    available: '利用可能',
    soon: '開発中',
  },
  cta: {
    title: 'ネイティブなものを作ろう',
    body: 'ビューを書いて、本物のアプリを出荷——アイドルウィンドウは本当にアイドルに',
    docs: 'ドキュメントを読む',
    star: 'GitHub でスターする',
  },
  footer: {
    license: 'MIT または Apache-2.0',
  },
}
