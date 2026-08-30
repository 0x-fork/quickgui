import type { en } from './en'

export const ja: typeof en = {
  meta: {
    title: 'QuickGUI — 変わった部分だけを描画するデスクトップ UI',
    description:
      'QuickGUI はダメージ駆動・GPU アクセラレーションの Rust 製デスクトップ GUI フレームワーク。GPUI スタイルの流暢な API、Flexbox と CSS Grid、リテインドな Unicode テキスト、ネイティブアクセシビリティ、有界の仮想スクロールを備えます。クリーンなウィンドウは眠り、アイドル時の描画は 0 フレームです。',
  },
  common: {
    skipToContent: 'コンテンツへスキップ',
    getStarted: 'はじめる',
    copy: '「{{text}}」をコピー',
    copied: 'コピーしました',
    language: '言語',
  },
  nav: {
    features: '特徴',
    code: 'コード',
    architecture: 'アーキテクチャ',
    quickstart: 'クイックスタート',
    docs: 'ドキュメント',
  },
  hero: {
    badge: 'v{{version}} / crates.io で公開中',
    title: '変わった部分だけを描画するデスクトップ UI。',
    sub: 'QuickGUI はダメージ駆動・GPU アクセラレーションの Rust 製 GUI フレームワーク。GPUI スタイルの流暢なビュー、Flexbox と CSS Grid、リテインドテキスト、ネイティブアクセシビリティ——そしてウィンドウはアイドル時 0 フレームで眠ります。',
    caption: 'ダメージ駆動ループのライブモデル——Increment をクリック',
  },
  stats: {
    idle: 'クリーン時のアイドルフレーム',
    scroll: '100,000 行のスクロール',
    cpu: 'p95 フレーム CPU',
    tests: 'コアスイートのテスト数',
  },
  features: {
    eyebrow: 'QuickGUI の理由',
    title: 'マシンに敬意を払うフレームワーク。',
    lead: 'QuickGUI はレイアウト、シェイプ済みテキスト、シーン、GPU キャッシュをフレーム間で保持し、ダメージ部分だけを再描画します。',
    alsoInTheBox: 'ほかにも同梱',
    items: {
      sleeps: {
        title: 'アイドル時はスリープ',
        body: 'クリーンなウィンドウは ControlFlow::Wait で待機し、アイドルフレームを描画しません。ホバー、スクロール、ドラッグは保持済みジオメトリを再ペイントするだけ——ビュー再構築もレイアウト再計算もなし。',
      },
      gpu: {
        title: 'ダメージ駆動の GPU レンダリング',
        body: 'インスタンス化シェイプ、キャッシュ済みテキスト、パス、画像、SVG、シャドウ、カスタム WGSL のための WGPU パイプライン。互換ウィンドウはデバイスとキューを共有します。',
      },
      layout: {
        title: '見慣れたレイアウト',
        body: 'Taffy の Flexbox と CSS Grid に Tailwind スタイルのヘルパー。親サイズのコンテナクエリは、割り当てられたボックスが変わったときだけ再宣言されます。',
      },
      text: {
        title: 'テキストは第一級市民',
        body: 'Cosmic Text によるリテインドな Unicode シェーピング：OpenType 機能、順序付きフォールバック、BiDi、省略記号と行クランプ、IME、選択、編集、アンドゥ。',
      },
      a11y: {
        title: '設計段階からアクセシブル',
        body: 'AccessKit ツリーが本物のロール、関係、アクションを公開。VoiceOver に見えるのは実際のコントロールです——コントロールの絵ではなく。',
      },
      virtual: {
        title: '有界の仮想化',
        body: '均一・実測可変高のリストは可視行だけをマウントし、論理アンカーを保持。100 万行のテーブルもツリーも有界のまま。',
      },
    },
    chips: {
      menus: 'ネイティブメニュー',
      dialogs: 'ダイアログとシート',
      popovers: 'NSPanel ポップオーバー',
      select: 'Select / オートコンプリート / Combobox',
      tabs: 'タブ',
      collections: '仮想テーブルとツリー',
      dnd: 'ドラッグ＆ドロップ',
      clipboard: 'クリップボード',
      notifications: '通知',
      tray: 'トレイアイコン',
      motion: 'スプリングとトランジション',
      shaders: 'カスタムシェーダー',
      tests: '決定論的テスト',
      inspector: 'リテインドツリーインスペクター',
    },
  },
  code: {
    eyebrow: 'View API',
    title: 'ビューはただのコード。',
    lead: '同じカウンターを 2 通りで：GPUI スタイルの流暢な Rust、または Bun 上でネイティブに動く Solid 2 JSX——webview も仮想 DOM もありません。',
    points: {
      builders: {
        title: 'ふつうのコード、流暢なビルダー',
        body: 'Tailwind 語彙のヘルパーによる JSX 風の組み立て：.flex_col().items_center().gap_3()。マクロ不要。',
      },
      listeners: {
        title: '型付きのビューローカルリスナー',
        body: 'cx.listener がイベントをビューの状態に束ねます。変更して、無効化して、おしまい——フレームワークはちょうど 1 フレームだけスケジュールします。',
      },
      paint: {
        title: 'ペイントのみのインタラクション状態',
        body: 'hover・active・focus・ドラッグのバリアントは保持済みジオメトリを再ペイント。.transition() を足せば再レイアウトなしで補間します。',
      },
    },
    viewApiDocs: 'View API ドキュメント',
    solidDocs: 'Solid レンダラーのドキュメント',
  },
  architecture: {
    eyebrow: 'アーキテクチャ',
    title: '何かが変わったときだけ、フレームは生まれる。',
    lead: '各ステージは入力が実際に変わるまで保持・再利用されます。何も変わらなければフレームは生成されません——クリーンなウィンドウは ControlFlow::Wait に停まります。',
    pipeline: {
      input: {
        title: '入力イベント',
        body: 'Winit + AccessKit。イベントループ境界で合流されます。',
      },
      tree: {
        title: 'リテインド要素ツリー',
        body: 'アプリの変更時のみ再構築。キー付きリスナーと状態は生き残ります。',
      },
      layout: {
        title: 'Taffy レイアウト',
        body: 'Flexbox、Grid、コンテナクエリ——レイアウト変更時だけ再実行。',
      },
      scene: {
        title: '再利用可能なシーン',
        body: '順序付きプレーンと z レイヤー。ホバーとスクロールはそのまま再利用。',
      },
      submit: {
        title: 'WGPU サブミット',
        body: 'インスタンス化シェイプ、キャッシュ済みグリフ、保持パス——有界の 1 パス。',
      },
    },
    docsLink: 'アーキテクチャドキュメントを読む',
    demoCaption: '同じ考え方をブラウザで——存在するのは可視行だけ',
    gatesTitle: '数字はゲートで検証済み。雰囲気ではなく。',
    gatesBody:
      '自己終了する macOS ゲートが実際の WindowServer 上で 100,000 行のリストをスクロールし、フレーム時間・CPU・メモリ・キャッシュ・アイドルの予算を強制します。',
    gates: {
      scroll: '持続双方向スクロール',
      frameCpu: 'p95 フレーム CPU',
      processCpu: 'プロセス全体の CPU',
      rss: 'ピーク RSS',
      idle: '余分なアイドルフレーム',
    },
    gatesLink: 'ゲートを自分で実行する',
  },
  statement: {
    text: 'デスクトップのピクセルの大半は、矩形、グリフ、アイコン、画像です。QuickGUI はパイプラインをまさにそのワークロードに捧げ——それ以外はすべて眠らせます。',
    link: 'このレンダラーである理由',
  },
  quickstart: {
    eyebrow: 'クイックスタート',
    title: 'ゼロからネイティブウィンドウへ。',
    lead: 'Rust クレートを直接使うか、Solid で書いて QuickGUI CLI で出荷を。',
    rust: {
      addCrate: 'クレートを追加',
      write: 'ビューを書く',
      writeBody:
        '上の<lnk>カウンター</lnk>は完全な <c>main.rs</c> です——流暢なビュー、型付きリスナー、マクロなし。',
      run: '実行',
      footnote:
        'Rust 2024 edition · macOS 0.1 検収済み · ドキュメントは <lnk>docs.rs</lnk>',
    },
    solid: {
      scaffold: 'プロジェクトを作って dev アプリを起動',
      ship: '署名済みビルドを出荷',
      body: '<c>quickgui dev</c> は本物の署名済み .app を実行し、編集のたびにホットリスタート。<c>quickgui build</c> は自己完結の .app とバージョン付き DMG を生成し、公証まで組み込みです。',
      footnote:
        'Bun 1.3+ · webview なし・仮想 DOM なし · <lnk>CLI ドキュメント</lnk>',
    },
  },
  platforms: {
    eyebrow: 'プラットフォーム',
    title: '動く場所について、正直に。',
    lead: 'マイルストーンは日付ではなく証拠でゲートされます。コンパイルできることを、ネイティブの証明に格上げすることはありません。',
    status: {
      accepted: '検収済み',
      compiles: 'コンパイル可',
    },
    items: {
      macos:
        '0.1 のプラットフォーム：ネイティブウィンドウ、メニュー、ダイアログ、NSPanel ポップオーバー、IME、VoiceOver 投影、ライブ性能ゲート。',
      windows:
        '現在は Winit + WGPU でコンパイル可能。ネイティブランタイム・ビジュアル・アクセシビリティの検収は 0.3 マイルストーンです。',
      linux:
        '現在は Winit + WGPU でコンパイル可能。ネイティブランタイム・ビジュアル・アクセシビリティの検収は 0.3 マイルストーンです。',
    },
    roadmapTitle: 'ロードマップ',
    shipped: '公開済み',
    roadmap: {
      m1: {
        label: 'macOS ファーストの基盤',
        detail: '2026-08-27 に crates.io へ公開。',
      },
      m2: {
        label: 'スタイルなしコンポーネント契約',
        detail:
          'ポップオーバーメニュー、Select、Combobox、ダイアログ、テーブル、ツリー——macOS で実機検収。',
      },
      m3: {
        label: 'Windows と Linux の同等性',
        detail:
          '両プラットフォームでのネイティブビジュアル、IME、アクセシビリティ、性能ゲート。',
      },
      m4: {
        label: '安定したクロスプラットフォーム契約',
        detail:
          '文書化された互換性ポリシーと、どこでも達成されるリソース予算。',
      },
    },
    statusLink: 'ステータスとロードマップの全文',
  },
  cta: {
    title: 'ネイティブな何かを作ろう。',
    body: 'クレートを追加し、ビューを書き、署名済みアプリを出荷——アイドルなウィンドウは、本当にアイドルに。',
    docs: 'ドキュメントを読む',
    star: 'GitHub でスターする',
  },
  footer: {
    description:
      'ダメージ駆動・GPU アクセラレーションの Rust 製デスクトップ GUI フレームワーク。',
    project: 'プロジェクト',
    packages: 'パッケージ',
    community: 'コミュニティ',
    links: {
      docs: 'ドキュメント',
      viewApi: 'View API',
      architecture: 'アーキテクチャ',
      status: 'ステータスとロードマップ',
      changelog: '変更履歴',
      crate: 'crates.io の quickgui',
      docsRs: 'docs.rs の API ドキュメント',
      solid: 'Solid 2 レンダラー',
      cli: 'CLI とパッケージング',
      github: 'GitHub',
      issues: 'Issues',
      examples: 'サンプル',
      license: 'ライセンス',
    },
    license: 'MIT または Apache-2.0（選択制）',
  },
}
