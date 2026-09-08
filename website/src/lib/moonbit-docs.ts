import type { DocsPageMeta } from "./docs";

export const MOONBIT_DOCS_PAGES: readonly DocsPageMeta[] = [
  {
    frontend: "moonbit",
    slug: "getting-started",
    title: "Getting Started",
    description: "Build a native app with MoonBit components and fast incremental builds.",
    outline: [
      { id: "requirements", title: "Requirements" },
      { id: "run-the-counter", title: "Run the counter" },
      { id: "your-first-window", title: "Your first window" },
      { id: "build-and-examples", title: "Build and examples" },
    ],
    searchTerms: [
      "moonbit",
      "install",
      "moon",
      "build",
      "counter",
      "quick git",
      "安装",
      "构建",
      "インストール",
      "ビルド",
    ],
    translations: {
      zh: {
        title: "入门",
        description: "使用 MoonBit 组件和快速增量构建开发原生应用。",
        outline: [
          { id: "环境要求", title: "环境要求" },
          { id: "运行计数器", title: "运行计数器" },
          { id: "第一个窗口", title: "第一个窗口" },
          { id: "构建与示例", title: "构建与示例" },
        ],
      },
      ja: {
        title: "はじめに",
        description: "MoonBit のコンポーネントと高速な増分ビルドでネイティブアプリを作成します。",
        outline: [
          { id: "必要な環境", title: "必要な環境" },
          { id: "カウンターを実行", title: "カウンターを実行" },
          { id: "最初のウィンドウ", title: "最初のウィンドウ" },
          { id: "ビルドとサンプル", title: "ビルドとサンプル" },
        ],
      },
    },
  },
  {
    frontend: "moonbit",
    slug: "project-structure",
    title: "Project Structure",
    description: "Configure MoonBit modules, the local SDK, and native packaging.",
    outline: [
      { id: "generated-files", title: "Generated files" },
      { id: "local-development", title: "Local development" },
      { id: "configuration", title: "Configuration" },
      { id: "packages", title: "Packages" },
    ],
    searchTerms: [
      "moon.pkg",
      "moon.mod",
      "moon.work",
      "config",
      "toml",
      "workspace",
      "项目",
      "配置",
      "設定",
    ],
    translations: {
      zh: {
        title: "项目结构",
        description: "配置 MoonBit 模块、本地 SDK 和原生打包。",
        outline: [
          { id: "生成的文件", title: "生成的文件" },
          { id: "本地开发", title: "本地开发" },
          { id: "配置", title: "配置" },
          { id: "包", title: "包" },
        ],
      },
      ja: {
        title: "プロジェクト構成",
        description: "MoonBit モジュール、ローカル SDK、ネイティブパッケージを設定します。",
        outline: [
          { id: "生成されるファイル", title: "生成されるファイル" },
          { id: "ローカル開発", title: "ローカル開発" },
          { id: "設定", title: "設定" },
          { id: "パッケージ", title: "パッケージ" },
        ],
      },
    },
  },
  {
    "frontend": "moonbit",
    "slug": "reactivity",
    "title": "Reactivity",
    "description": "Signals connect state to the text and properties that read it. Updates change retained nodes without rerunning the entire component.",
    "outline": [
      {
        "id": "live-bindings",
        "title": "Live bindings"
      },
      {
        "id": "derived-state-and-cleanup",
        "title": "Derived state and cleanup"
      }
    ],
    "searchTerms": [
      "signal",
      "reactivity",
      "memo",
      "effect",
      "batch",
      "cleanup"
    ],
    "translations": {
      "zh": {
        "title": "响应式",
        "description": "信号将状态连接到读取它的文本和属性。更新只改变保留的节点，不会重新执行整个组件。",
        "outline": [
          {
            "id": "实时绑定",
            "title": "实时绑定"
          },
          {
            "id": "派生状态与清理",
            "title": "派生状态与清理"
          }
        ]
      },
      "ja": {
        "title": "リアクティビティ",
        "description": "シグナルは状態を、それを読むテキストやプロパティに結び付けます。更新時は保持されたノードだけを変更し、コンポーネント全体を再実行しません。",
        "outline": [
          {
            "id": "動的バインディング",
            "title": "動的バインディング"
          },
          {
            "id": "派生状態とクリーンアップ",
            "title": "派生状態とクリーンアップ"
          }
        ]
      }
    }
  },
  {
    "frontend": "moonbit",
    "slug": "rendering",
    "title": "Rendering",
    "description": "Components construct a retained tree once when mounted. Reactive bindings update the affected nodes; the native core handles layout, painting, and accessibility.",
    "outline": [
      {
        "id": "view-syntax",
        "title": "View syntax"
      },
      {
        "id": "conditional-children",
        "title": "Conditional children"
      },
      {
        "id": "keyed-children",
        "title": "Keyed children"
      },
      {
        "id": "checking-testing-and-debugging",
        "title": "Checking, testing, and debugging"
      }
    ],
    "searchTerms": [
      "rendering",
      "retained",
      "children",
      "mount",
      "lifecycle",
      "keyed"
    ],
    "translations": {
      "zh": {
        "title": "渲染",
        "description": "组件在挂载时创建保留树。响应式绑定更新受影响的节点，原生核心负责布局、绘制和无障碍行为。",
        "outline": [
          {
            "id": "视图语法",
            "title": "视图语法"
          },
          {
            "id": "条件子节点",
            "title": "条件子节点"
          },
          {
            "id": "按键保留子节点",
            "title": "按键保留子节点"
          },
          {
            "id": "检查测试与调试",
            "title": "检查、测试与调试"
          }
        ]
      },
      "ja": {
        "title": "レンダリング",
        "description": "コンポーネントはマウント時に保持ツリーを作成します。リアクティブなバインディングが対象ノードを更新し、レイアウト、描画、アクセシビリティはネイティブコアが担当します。",
        "outline": [
          {
            "id": "ビューの構文",
            "title": "ビューの構文"
          },
          {
            "id": "条件付きの子ノード",
            "title": "条件付きの子ノード"
          },
          {
            "id": "キー付きの子ノード",
            "title": "キー付きの子ノード"
          },
          {
            "id": "チェックテストデバッグ",
            "title": "チェック・テスト・デバッグ"
          }
        ]
      }
    }
  },
  {
    "frontend": "moonbit",
    "slug": "routing",
    "title": "Routing",
    "description": "The router selects components from the current application path and keeps a navigation history. Routes render native QuickGUI content in the current window.",
    "outline": [
      {
        "id": "nested-layouts",
        "title": "Nested layouts"
      },
      {
        "id": "navigation",
        "title": "Navigation"
      },
      {
        "id": "retained-outlets",
        "title": "Retained outlets"
      }
    ],
    "searchTerms": [
      "router",
      "route",
      "layout",
      "outlet",
      "navigation",
      "history",
      "parameters"
    ],
    "translations": {
      "zh": {
        "title": "路由",
        "description": "路由根据应用当前路径选择组件并维护导航历史。路由内容在当前窗口中以原生 QuickGUI 节点渲染。",
        "outline": [
          {
            "id": "嵌套布局",
            "title": "嵌套布局"
          },
          {
            "id": "导航",
            "title": "导航"
          },
          {
            "id": "保留的路由出口",
            "title": "保留的路由出口"
          }
        ]
      },
      "ja": {
        "title": "ルーティング",
        "description": "ルーターは現在のアプリ内パスからコンポーネントを選び、ナビゲーション履歴を管理します。ルートは現在のウィンドウにネイティブ QuickGUI コンテンツを描画します。",
        "outline": [
          {
            "id": "入れ子のレイアウト",
            "title": "入れ子のレイアウト"
          },
          {
            "id": "ナビゲーション",
            "title": "ナビゲーション"
          },
          {
            "id": "保持されるアウトレット",
            "title": "保持されるアウトレット"
          }
        ]
      }
    }
  },
  {
    frontend: "moonbit",
    slug: "styling",
    title: "Styling & Layout",
    description: "Compose fluent styles, merge declarations, and bind native state styles.",
    outline: [
      { id: "fluent-layout", title: "Fluent layout" },
      { id: "merging-and-reactive-styles", title: "Merging and reactive styles" },
      { id: "interaction-states", title: "Interaction states" },
      { id: "colors-and-svg", title: "Colors and SVG" },
    ],
    searchTerms: [
      "style",
      "fluent",
      "merge",
      "flexbox",
      "color",
      "hover",
      "layout",
      "样式",
      "布局",
      "スタイル",
    ],
    translations: {
      zh: {
        title: "样式与布局",
        description: "组合链式样式、合并声明，并绑定原生状态样式。",
        outline: [
          { id: "链式布局", title: "链式布局" },
          { id: "合并与响应式样式", title: "合并与响应式样式" },
          { id: "交互状态", title: "交互状态" },
          { id: "颜色与-svg", title: "颜色与 SVG" },
        ],
      },
      ja: {
        title: "スタイルとレイアウト",
        description: "fluent スタイルを組み合わせ、宣言をマージし、状態スタイルをバインドします。",
        outline: [
          { id: "fluent-レイアウト", title: "fluent レイアウト" },
          { id: "マージとリアクティブなスタイル", title: "マージとリアクティブなスタイル" },
          { id: "操作状態", title: "操作状態" },
          { id: "色と-svg", title: "色と SVG" },
        ],
      },
    },
  },
  {
    frontend: "moonbit",
    slug: "animations",
    title: "Transitions & Animation",
    description: "Animate retained paint properties without per-frame MoonBit work.",
    outline: [
      { id: "hover-transitions", title: "Hover transitions" },
      { id: "timing", title: "Timing" },
      { id: "supported-properties", title: "Supported properties" },
      { id: "lifetime-and-reduced-motion", title: "Lifetime and reduced motion" },
    ],
    searchTerms: [
      "transition",
      "animation",
      "easing",
      "duration",
      "transform",
      "reduced motion",
      "动画",
      "过渡",
      "アニメーション",
    ],
    translations: {
      zh: {
        title: "过渡与动画",
        description: "为保留的绘制属性添加动画，无需 MoonBit 逐帧更新。",
        outline: [
          { id: "悬停过渡", title: "悬停过渡" },
          { id: "时间配置", title: "时间配置" },
          { id: "支持的属性", title: "支持的属性" },
          { id: "生命周期与减少动态效果", title: "生命周期与减少动态效果" },
        ],
      },
      ja: {
        title: "トランジションとアニメーション",
        description:
          "MoonBit で毎フレーム処理せず、保持された描画プロパティをアニメーションにします。",
        outline: [
          { id: "ホバーのトランジション", title: "ホバーのトランジション" },
          { id: "時間の設定", title: "時間の設定" },
          { id: "対応するプロパティ", title: "対応するプロパティ" },
          { id: "ライフタイムと視差効果の軽減", title: "ライフタイムと視差効果の軽減" },
        ],
      },
    },
  },
  {
    frontend: "moonbit",
    slug: "components",
    title: "Components",
    description: "Use MoonBit primitives, native table parts, and resizable panels.",
    outline: [
      { id: "defining-components", title: "Defining components" },
      { id: "interactive-gallery", title: "Interactive gallery" },
      { id: "primitives", title: "Primitives" },
      { id: "tables-and-visible-rows", title: "Tables and visible rows" },
      { id: "resizable-panels", title: "Resizable panels" },
      { id: "compound-parts", title: "Compound parts" },
    ],
    searchTerms: [
      "components",
      "table",
      "virtualization",
      "visibleRange",
      "splitter",
      "resizable_panel",
      "svg",
      "markdown",
      "表格",
      "组件",
      "テーブル",
    ],
    translations: {
      zh: {
        title: "组件",
        description: "使用 MoonBit 基础组件、原生表格部件和可调整面板。",
        outline: [
          { id: "定义组件", title: "定义组件" },
          { id: "交互式组件示例", title: "交互式组件示例" },
          { id: "基础组件", title: "基础组件" },
          { id: "表格与可见行", title: "表格与可见行" },
          { id: "可调整面板", title: "可调整面板" },
          { id: "复合部件", title: "复合部件" },
        ],
      },
      ja: {
        title: "コンポーネント",
        description:
          "MoonBit のプリミティブ、ネイティブテーブルのパーツ、サイズ変更可能なパネルを使います。",
        outline: [
          { id: "コンポーネントの定義", title: "コンポーネントの定義" },
          { id: "対話式ギャラリー", title: "対話式ギャラリー" },
          { id: "プリミティブ", title: "プリミティブ" },
          { id: "テーブルと可視行", title: "テーブルと可視行" },
          { id: "サイズ変更可能なパネル", title: "サイズ変更可能なパネル" },
          { id: "複合パーツ", title: "複合パーツ" },
        ],
      },
    },
  },
  {
    frontend: "moonbit",
    slug: "forms-and-input",
    title: "Forms & Input",
    description: "Bind text fields, submit handlers, and disabled controls to signals.",
    outline: [
      { id: "controlled-input", title: "Controlled input" },
      { id: "disabled-and-read-only", title: "Disabled and read-only" },
      { id: "other-controls", title: "Other controls" },
    ],
    searchTerms: [
      "input",
      "form",
      "submit",
      "bind_value",
      "disabled",
      "readonly",
      "输入",
      "表单",
      "フォーム",
    ],
    translations: {
      zh: {
        title: "表单与输入",
        description: "将文本字段、提交处理和禁用状态绑定到信号。",
        outline: [
          { id: "受控输入", title: "受控输入" },
          { id: "禁用与只读", title: "禁用与只读" },
          { id: "其他控件", title: "其他控件" },
        ],
      },
      ja: {
        title: "フォームと入力",
        description: "テキスト入力、送信処理、無効状態をシグナルにバインドします。",
        outline: [
          { id: "制御された入力", title: "制御された入力" },
          { id: "無効と読み取り専用", title: "無効と読み取り専用" },
          { id: "その他のコントロール", title: "その他のコントロール" },
        ],
      },
    },
  },
  {
    frontend: "moonbit",
    slug: "overlays-and-dialogs",
    title: "Overlays & Dialogs",
    description: "Open system dialogs and native menus from MoonBit components.",
    outline: [
      { id: "choose-a-directory", title: "Choose a directory" },
      { id: "popup-and-context-menus", title: "Popup and context menus" },
      { id: "application-menus-and-overlays", title: "Application menus and overlays" },
    ],
    searchTerms: [
      "menu",
      "dialog",
      "file picker",
      "popup_menu",
      "context menu",
      "菜单",
      "对话框",
      "メニュー",
      "ダイアログ",
    ],
    translations: {
      zh: {
        title: "浮层与对话框",
        description: "从 MoonBit 组件打开系统对话框与原生菜单。",
        outline: [
          { id: "选择目录", title: "选择目录" },
          { id: "弹出与上下文菜单", title: "弹出与上下文菜单" },
          { id: "应用菜单与浮层", title: "应用菜单与浮层" },
        ],
      },
      ja: {
        title: "オーバーレイとダイアログ",
        description:
          "MoonBit のコンポーネントからシステムダイアログとネイティブメニューを開きます。",
        outline: [
          { id: "ディレクトリを選択", title: "ディレクトリを選択" },
          { id: "ポップアップとコンテキストメニュー", title: "ポップアップとコンテキストメニュー" },
          {
            id: "アプリケーションメニューとオーバーレイ",
            title: "アプリケーションメニューとオーバーレイ",
          },
        ],
      },
    },
  },
  {
    frontend: "moonbit",
    slug: "native-services",
    title: "Windows & Native Services",
    description: "Manage window lifetimes and asynchronous native work.",
    outline: [
      { id: "window-lifetime", title: "Window lifetime" },
      { id: "commands-and-services", title: "Commands and services" },
      { id: "events-and-file-watching", title: "Events and file watching" },
    ],
    searchTerms: [
      "window",
      "native",
      "lifecycle",
      "invoke",
      "watch_files",
      "with_owner",
      "窗口",
      "服务",
      "ウィンドウ",
    ],
    translations: {
      zh: {
        title: "窗口与原生服务",
        description: "管理窗口生命周期与异步原生操作。",
        outline: [
          { id: "窗口生命周期", title: "窗口生命周期" },
          { id: "命令与服务", title: "命令与服务" },
          { id: "事件与文件监听", title: "事件与文件监听" },
        ],
      },
      ja: {
        title: "ウィンドウとネイティブサービス",
        description: "ウィンドウのライフタイムと非同期のネイティブ処理を管理します。",
        outline: [
          { id: "ウィンドウのライフタイム", title: "ウィンドウのライフタイム" },
          { id: "コマンドとサービス", title: "コマンドとサービス" },
          { id: "イベントとファイル監視", title: "イベントとファイル監視" },
        ],
      },
    },
  },
  {
    frontend: "moonbit",
    slug: "extensions",
    title: "Authoring Extensions",
    description: "Package independent native providers and call them from MoonBit.",
    outline: [
      { id: "create-a-provider", title: "Create a provider" },
      { id: "select-and-package", title: "Select and package" },
      { id: "invoke-a-service", title: "Invoke a service" },
      { id: "distribution-and-examples", title: "Distribution and examples" },
    ],
    searchTerms: [
      "extension",
      "authoring",
      "provider",
      "init-extension",
      "rust",
      "zig",
      "pure go",
      "manifest",
      "ABI",
      "扩展",
      "拡張",
    ],
    translations: {
      zh: {
        title: "编写扩展",
        description: "打包独立原生后端并从 MoonBit 调用。",
        outline: [
          { id: "创建原生后端", title: "创建原生后端" },
          { id: "选择与打包", title: "选择与打包" },
          { id: "调用服务", title: "调用服务" },
          { id: "分发与示例", title: "分发与示例" },
        ],
      },
      ja: {
        title: "拡張の作成",
        description: "独立したネイティブプロバイダーをパッケージ化し、MoonBit から呼び出します。",
        outline: [
          { id: "プロバイダーを作成", title: "プロバイダーを作成" },
          { id: "選択とパッケージ化", title: "選択とパッケージ化" },
          { id: "サービスを呼び出す", title: "サービスを呼び出す" },
          { id: "配布とサンプル", title: "配布とサンプル" },
        ],
      },
    },
  },
  {
    frontend: "moonbit",
    slug: "swift-ui",
    title: "SwiftUI",
    description: "Host native macOS controls from MoonBit.",
    outline: [
      {
        id: "host-a-control",
        title: "Host a control",
      },
      {
        id: "controlled-values",
        title: "Controlled values",
      },
      {
        id: "native-modifiers",
        title: "Native modifiers",
      },
    ],
    searchTerms: ["swift-ui", "moonbit"],
    translations: {
      zh: {
        title: "SwiftUI",
        description: "从 MoonBit 托管原生 macOS 控件。",
        outline: [
          {
            id: "托管控件",
            title: "托管控件",
          },
          {
            id: "受控值",
            title: "受控值",
          },
          {
            id: "原生修饰符",
            title: "原生修饰符",
          },
        ],
      },
      ja: {
        title: "SwiftUI",
        description: "MoonBit から macOS のネイティブコントロールをホストします。",
        outline: [
          {
            id: "コントロールをホスト",
            title: "コントロールをホスト",
          },
          {
            id: "制御値",
            title: "制御値",
          },
          {
            id: "ネイティブ修飾子",
            title: "ネイティブ修飾子",
          },
        ],
      },
    },
  },
  {
    frontend: "moonbit",
    slug: "swift-ui-hosting",
    title: "Modifiers & Hosting",
    description: "Compose SwiftUI and QuickGUI subtrees with explicit host boundaries.",
    outline: [
      {
        id: "host-boundaries",
        title: "Host boundaries",
      },
      {
        id: "reverse-hosting",
        title: "Reverse hosting",
      },
      {
        id: "lifetime",
        title: "Lifetime",
      },
    ],
    searchTerms: ["swift-ui-hosting", "moonbit"],
    translations: {
      zh: {
        title: "修饰器与托管",
        description: "在明确的宿主边界内组合 SwiftUI 和 QuickGUI 子树。",
        outline: [
          {
            id: "宿主边界",
            title: "宿主边界",
          },
          {
            id: "反向托管",
            title: "反向托管",
          },
          {
            id: "生命周期",
            title: "生命周期",
          },
        ],
      },
      ja: {
        title: "モディファイアとホスティング",
        description: "ホスト境界を明確にして SwiftUI と QuickGUI を組み合わせます。",
        outline: [
          {
            id: "ホスト境界",
            title: "ホスト境界",
          },
          {
            id: "逆方向のホスティング",
            title: "逆方向のホスティング",
          },
          {
            id: "ライフサイクル",
            title: "ライフサイクル",
          },
        ],
      },
    },
  },
  {
    frontend: "moonbit",
    slug: "updater",
    title: "Auto Updater",
    description: "Package the optional updater service for a MoonBit application.",
    outline: [
      {
        id: "configuration",
        title: "Configuration",
      },
      {
        id: "native-extension",
        title: "Native extension",
      },
      {
        id: "release-artifacts",
        title: "Release artifacts",
      },
    ],
    searchTerms: ["updater", "moonbit"],
    translations: {
      zh: {
        title: "自动更新",
        description: "为 MoonBit 应用打包可选更新服务。",
        outline: [
          {
            id: "配置",
            title: "配置",
          },
          {
            id: "原生扩展",
            title: "原生扩展",
          },
          {
            id: "发布产物",
            title: "发布产物",
          },
        ],
      },
      ja: {
        title: "自動更新",
        description: "MoonBit アプリ向けに任意の更新サービスをパッケージ化します。",
        outline: [
          {
            id: "設定",
            title: "設定",
          },
          {
            id: "ネイティブ拡張",
            title: "ネイティブ拡張",
          },
          {
            id: "リリース成果物",
            title: "リリース成果物",
          },
        ],
      },
    },
  },
];
