import { docsOutline, docsTitle } from './docs-structure'
import type { DocsPageMeta } from './docs'

export const MOONBIT_DOCS_PAGES: readonly DocsPageMeta[] = [
  {
    frontend: 'moonbit',
    slug: 'getting-started',
    outline: docsOutline('getting-started'),
    title: docsTitle('getting-started'),
    description: 'Build a native app with MoonBit components and fast incremental builds.',
    searchTerms: [
      'moonbit',
      'install',
      'moon',
      'build',
      'counter',
      'quick git',
      '安装',
      '构建',
      'インストール',
      'ビルド',
    ],
    translations: {
      zh: {
        description: '使用 MoonBit 组件和快速增量构建开发原生应用。',
      },
      ja: {
        description: 'MoonBit のコンポーネントと高速な増分ビルドでネイティブアプリを作成します。',
      },
    },
  },
  {
    frontend: 'moonbit',
    slug: 'project-structure',
    outline: docsOutline('project-structure'),
    title: docsTitle('project-structure'),
    description: 'Configure MoonBit modules, the local SDK, and native packaging.',
    searchTerms: [
      'moon.pkg',
      'moon.mod',
      'moon.work',
      'config',
      'toml',
      'workspace',
      '项目',
      '配置',
      '設定',
    ],
    translations: {
      zh: {
        description: '配置 MoonBit 模块、本地 SDK 和原生打包。',
      },
      ja: {
        description: 'MoonBit モジュール、ローカル SDK、ネイティブパッケージを設定します。',
      },
    },
  },
  {
    frontend: 'moonbit',
    slug: 'updater',
    outline: docsOutline('updater'),
    title: docsTitle('updater'),
    description: 'Package the optional updater service for a MoonBit application.',
    searchTerms: ['updater', 'moonbit'],
    translations: {
      zh: {
        description: '为 MoonBit 应用打包可选更新服务。',
      },
      ja: {
        description: 'MoonBit アプリ向けに任意の更新サービスをパッケージ化します。',
      },
    },
  },
  {
    frontend: 'moonbit',
    slug: 'extensions',
    outline: docsOutline('extensions'),
    title: docsTitle('extensions'),
    description: 'Package independent native providers and call them from MoonBit.',
    searchTerms: [
      'extension',
      'authoring',
      'provider',
      'init-extension',
      'rust',
      'zig',
      'pure go',
      'manifest',
      'ABI',
      '扩展',
      '拡張',
    ],
    translations: {
      zh: {
        description: '打包独立原生后端并从 MoonBit 调用。',
      },
      ja: {
        description: '独立したネイティブプロバイダーをパッケージ化し、MoonBit から呼び出します。',
      },
    },
  },
  {
    frontend: 'moonbit',
    slug: 'native-services',
    outline: docsOutline('native-services'),
    title: docsTitle('native-services'),
    description: 'Manage window lifetimes and asynchronous native work.',
    searchTerms: [
      'window',
      'native',
      'lifecycle',
      'invoke',
      'watch_files',
      'with_owner',
      '窗口',
      '服务',
      'ウィンドウ',
    ],
    translations: {
      zh: {
        description: '管理窗口生命周期与异步原生操作。',
      },
      ja: {
        description: 'ウィンドウのライフタイムと非同期のネイティブ処理を管理します。',
      },
    },
  },
  {
    frontend: 'moonbit',
    slug: 'reactivity',
    outline: docsOutline('reactivity'),
    title: docsTitle('reactivity'),
    description:
      'Signals connect state to the text and properties that read it. Updates change retained nodes without rerunning the entire component.',
    searchTerms: ['signal', 'reactivity', 'memo', 'effect', 'batch', 'cleanup'],
    translations: {
      zh: {
        description:
          '信号将状态连接到读取它的文本和属性。更新只改变保留的节点，不会重新执行整个组件。',
      },
      ja: {
        description:
          'シグナルは状態を、それを読むテキストやプロパティに結び付けます。更新時は保持されたノードだけを変更し、コンポーネント全体を再実行しません。',
      },
    },
  },
  {
    frontend: 'moonbit',
    slug: 'rendering',
    outline: docsOutline('rendering'),
    title: docsTitle('rendering'),
    description:
      'Components construct a retained tree once when mounted. Reactive bindings update the affected nodes; the native core handles layout, painting, and accessibility.',
    searchTerms: ['rendering', 'retained', 'children', 'mount', 'lifecycle', 'keyed'],
    translations: {
      zh: {
        description:
          '组件在挂载时创建保留树。响应式绑定更新受影响的节点，原生核心负责布局、绘制和无障碍行为。',
      },
      ja: {
        description:
          'コンポーネントはマウント時に保持ツリーを作成します。リアクティブなバインディングが対象ノードを更新し、レイアウト、描画、アクセシビリティはネイティブコアが担当します。',
      },
    },
  },
  {
    frontend: 'moonbit',
    slug: 'components',
    outline: docsOutline('components'),
    title: docsTitle('components'),
    description: 'Use MoonBit primitives, native table parts, and resizable panels.',
    searchTerms: [
      'components',
      'table',
      'virtualization',
      'visibleRange',
      'splitter',
      'resizable_panel',
      'svg',
      'markdown',
      '表格',
      '组件',
      'テーブル',
    ],
    translations: {
      zh: {
        description: '使用 MoonBit 基础组件、原生表格部件和可调整面板。',
      },
      ja: {
        description:
          'MoonBit のプリミティブ、ネイティブテーブルのパーツ、サイズ変更可能なパネルを使います。',
      },
    },
  },
  {
    frontend: 'moonbit',
    slug: 'routing',
    outline: docsOutline('routing'),
    title: docsTitle('routing'),
    description:
      'The router selects components from the current application path and keeps a navigation history. Routes render native QuickGUI content in the current window.',
    searchTerms: ['router', 'route', 'layout', 'outlet', 'navigation', 'history', 'parameters'],
    translations: {
      zh: {
        description:
          '路由根据应用当前路径选择组件并维护导航历史。路由内容在当前窗口中以原生 QuickGUI 节点渲染。',
      },
      ja: {
        description:
          'ルーターは現在のアプリ内パスからコンポーネントを選び、ナビゲーション履歴を管理します。ルートは現在のウィンドウにネイティブ QuickGUI コンテンツを描画します。',
      },
    },
  },
  {
    frontend: 'moonbit',
    slug: 'styling',
    outline: docsOutline('styling'),
    title: docsTitle('styling'),
    description: 'Compose fluent styles, merge declarations, and bind native state styles.',
    searchTerms: [
      'style',
      'fluent',
      'merge',
      'flexbox',
      'color',
      'hover',
      'layout',
      '样式',
      '布局',
      'スタイル',
    ],
    translations: {
      zh: {
        description: '组合链式样式、合并声明，并绑定原生状态样式。',
      },
      ja: {
        description: 'fluent スタイルを組み合わせ、宣言をマージし、状態スタイルをバインドします。',
      },
    },
  },
  {
    frontend: 'moonbit',
    slug: 'animations',
    outline: docsOutline('animations'),
    title: docsTitle('animations'),
    description: 'Animate retained paint properties without per-frame MoonBit work.',
    searchTerms: [
      'transition',
      'animation',
      'easing',
      'duration',
      'transform',
      'reduced motion',
      '动画',
      '过渡',
      'アニメーション',
    ],
    translations: {
      zh: {
        description: '为保留的绘制属性添加动画，无需 MoonBit 逐帧更新。',
      },
      ja: {
        description:
          'MoonBit で毎フレーム処理せず、保持された描画プロパティをアニメーションにします。',
      },
    },
  },
  {
    frontend: 'moonbit',
    slug: 'forms-and-input',
    outline: docsOutline('forms-and-input'),
    title: docsTitle('forms-and-input'),
    description: 'Bind text fields, submit handlers, and disabled controls to signals.',
    searchTerms: [
      'input',
      'form',
      'submit',
      'bind_value',
      'disabled',
      'readonly',
      '输入',
      '表单',
      'フォーム',
    ],
    translations: {
      zh: {
        description: '将文本字段、提交处理和禁用状态绑定到信号。',
      },
      ja: {
        description: 'テキスト入力、送信処理、無効状態をシグナルにバインドします。',
      },
    },
  },
  {
    frontend: 'moonbit',
    slug: 'overlays-and-dialogs',
    outline: docsOutline('overlays-and-dialogs'),
    title: docsTitle('overlays-and-dialogs'),
    description: 'Open system dialogs and native menus from MoonBit components.',
    searchTerms: [
      'menu',
      'dialog',
      'file picker',
      'popup_menu',
      'context menu',
      '菜单',
      '对话框',
      'メニュー',
      'ダイアログ',
    ],
    translations: {
      zh: {
        description: '从 MoonBit 组件打开系统对话框与原生菜单。',
      },
      ja: {
        description:
          'MoonBit のコンポーネントからシステムダイアログとネイティブメニューを開きます。',
      },
    },
  },
  {
    frontend: 'moonbit',
    slug: 'swift-ui',
    outline: docsOutline('swift-ui'),
    title: docsTitle('swift-ui'),
    description: 'Host native macOS controls from MoonBit.',
    searchTerms: ['swift-ui', 'moonbit'],
    translations: {
      zh: {
        description: '从 MoonBit 托管原生 macOS 控件。',
      },
      ja: {
        description: 'MoonBit から macOS のネイティブコントロールをホストします。',
      },
    },
  },
  {
    frontend: 'moonbit',
    slug: 'swift-ui-hosting',
    outline: docsOutline('swift-ui-hosting'),
    title: docsTitle('swift-ui-hosting'),
    description: 'Compose SwiftUI and QuickGUI subtrees with explicit host boundaries.',
    searchTerms: ['swift-ui-hosting', 'moonbit'],
    translations: {
      zh: {
        description: '在明确的宿主边界内组合 SwiftUI 和 QuickGUI 子树。',
      },
      ja: {
        description: 'ホスト境界を明確にして SwiftUI と QuickGUI を組み合わせます。',
      },
    },
  },
]
