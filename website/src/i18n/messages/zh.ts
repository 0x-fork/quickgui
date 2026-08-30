import type { en } from './en'

export const zh: typeof en = {
  meta: {
    title: 'QuickGUI — 只渲染变化之处的桌面 UI',
    description:
      'QuickGUI 是一个损伤驱动（damage-driven）、GPU 加速的 Rust 桌面 GUI 框架：GPUI 风格的链式 API、Flexbox 与 CSS Grid、保留式 Unicode 文本、原生无障碍支持，以及有界虚拟滚动。干净的窗口会休眠，空闲应用零帧渲染。',
  },
  common: {
    skipToContent: '跳转到正文',
    getStarted: '快速开始',
    copy: '复制“{{text}}”',
    copied: '已复制',
    language: '语言',
  },
  nav: {
    features: '特性',
    code: '代码',
    architecture: '架构',
    quickstart: '快速开始',
    docs: '文档',
  },
  hero: {
    badge: 'v{{version}} / 已发布至 crates.io',
    title: '只渲染变化之处的桌面 UI。',
    sub: 'QuickGUI 是一个损伤驱动、GPU 加速的 Rust GUI 框架：GPUI 风格的链式视图、Flexbox 与 CSS Grid、保留式文本、原生无障碍——空闲窗口以零帧休眠。',
    caption: '损伤驱动渲染循环的实时演示——点击 Increment 试试',
  },
  stats: {
    idle: '干净窗口的空闲帧数',
    scroll: '滚动 100,000 行',
    cpu: 'p95 帧 CPU',
    tests: '核心测试套件用例',
  },
  features: {
    eyebrow: '为什么选 QuickGUI',
    title: '一个尊重硬件的框架。',
    lead: 'QuickGUI 在帧与帧之间保留布局、成形文本、场景与 GPU 缓存——只重绘损伤区域。',
    alsoInTheBox: '同样内置',
    items: {
      sleeps: {
        title: '空闲即休眠',
        body: '干净的窗口停在 ControlFlow::Wait，不渲染任何空闲帧。悬停、滚动与拖拽只重绘保留几何——不重建视图，不重跑布局。',
      },
      gpu: {
        title: '损伤驱动的 GPU 渲染',
        body: '基于 WGPU 的实例化图形、缓存文本、路径、图片、SVG、阴影与自定义 WGSL 管线。兼容窗口共享同一 device 与 queue。',
      },
      layout: {
        title: '你早已熟悉的布局',
        body: 'Taffy Flexbox 与 CSS Grid，配 Tailwind 风格的辅助方法；父尺寸容器查询只在被分配的盒子变化时重新声明。',
      },
      text: {
        title: '文本是一等公民',
        body: '基于 Cosmic Text 的保留式 Unicode 成形：OpenType 特性、有序回退、双向文本、省略与行钳制、IME、选区、编辑与撤销。',
      },
      a11y: {
        title: '天生可访问',
        body: 'AccessKit 树暴露真实的角色、关系与操作。VoiceOver 看到的是真控件——而不是一张控件的图片。',
      },
      virtual: {
        title: '有界虚拟化',
        body: '等高与实测变高列表只挂载可见行并保持逻辑锚点。百万行的表格与树依然有界。',
      },
    },
    chips: {
      menus: '原生菜单',
      dialogs: '对话框与 Sheet',
      popovers: 'NSPanel 弹出层',
      select: 'Select / 自动补全 / Combobox',
      tabs: '标签页',
      collections: '虚拟表格与树',
      dnd: '拖放',
      clipboard: '剪贴板',
      notifications: '通知',
      tray: '托盘图标',
      motion: '弹簧与过渡动画',
      shaders: '自定义着色器',
      tests: '确定性测试',
      inspector: '保留树检查器',
    },
  },
  code: {
    eyebrow: 'View API',
    title: '视图就是普通代码。',
    lead: '同一个计数器的两种写法：GPUI 风格的链式 Rust，或原生跑在 Bun 上的 Solid 2 JSX——没有 webview，没有虚拟 DOM。',
    points: {
      builders: {
        title: '普通代码，链式构建',
        body: '类 JSX 的组合方式加 Tailwind 词汇的辅助方法：.flex_col().items_center().gap_3()，无需宏。',
      },
      listeners: {
        title: '类型安全的视图本地监听器',
        body: 'cx.listener 把事件绑定到视图状态。修改、失效、完成——框架恰好调度一帧。',
      },
      paint: {
        title: '仅重绘的交互状态',
        body: 'hover、active、focus 与拖拽变体只重绘保留几何；加上 .transition() 即可在不重排的情况下平滑过渡。',
      },
    },
    viewApiDocs: 'View API 文档',
    solidDocs: 'Solid 渲染器文档',
  },
  architecture: {
    eyebrow: '架构',
    title: '只有发生变化，才会产生一帧。',
    lead: '每一级都被保留复用，直到其输入真正变化。若无变化，就不产出帧——干净的窗口停在 ControlFlow::Wait。',
    pipeline: {
      input: {
        title: '输入事件',
        body: 'Winit + AccessKit，在事件循环边界合并。',
      },
      tree: {
        title: '保留式元素树',
        body: '只因应用状态变化而重建；键控监听器与状态得以保留。',
      },
      layout: {
        title: 'Taffy 布局',
        body: 'Flexbox、Grid 与容器查询——只在布局变化时重跑。',
      },
      scene: {
        title: '可复用场景',
        body: '有序的平面与 z 层；悬停与滚动直接复用。',
      },
      submit: {
        title: 'WGPU 提交',
        body: '实例化图形、缓存字形、保留路径——一次有界提交。',
      },
    },
    docsLink: '阅读架构文档',
    demoCaption: '同样的思路，在你的浏览器里——只有可见行真实存在',
    gatesTitle: '数字经过验收门槛，不是感觉。',
    gatesBody:
      '自终止的 macOS 验收脚本在真实 WindowServer 上滚动 100,000 行列表，并强制执行帧时间、CPU、内存、缓存与空闲预算。',
    gates: {
      scroll: '持续双向滚动',
      frameCpu: 'p95 帧 CPU',
      processCpu: '整进程 CPU',
      rss: '峰值 RSS',
      idle: '额外空闲帧',
    },
    gatesLink: '亲自运行验收脚本',
  },
  statement: {
    text: '桌面上的大多数像素，无非矩形、字形、图标与图片。QuickGUI 把渲染管线专门献给这类负载——其余的一切，让它休眠。',
    link: '为什么这样设计渲染器',
  },
  quickstart: {
    eyebrow: '快速开始',
    title: '从零到一个原生窗口。',
    lead: '直接使用 Rust crate，或用 Solid 编写并通过 QuickGUI CLI 发布。',
    rust: {
      addCrate: '添加 crate',
      write: '编写一个视图',
      writeBody:
        '上面的<lnk>计数器</lnk>就是一个完整的 <c>main.rs</c>——链式视图、类型安全监听器、无宏。',
      run: '运行',
      footnote:
        'Rust 2024 edition · macOS 0.1 已验收 · 文档见 <lnk>docs.rs</lnk>',
    },
    solid: {
      scaffold: '创建项目并运行开发应用',
      ship: '发布签名构建',
      body: '<c>quickgui dev</c> 运行真实签名的 .app 并在编辑时热重启；<c>quickgui build</c> 产出自包含的 .app 与带版本号的 DMG，内置公证流程。',
      footnote: 'Bun 1.3+ · 无 webview、无虚拟 DOM · <lnk>CLI 文档</lnk>',
    },
  },
  platforms: {
    eyebrow: '平台',
    title: '坦诚交代它跑在哪里。',
    lead: '里程碑以证据为门槛，而非日期。能编译，永远不等于原生可用。',
    status: {
      accepted: '已验收',
      compiles: '可编译',
    },
    items: {
      macos:
        '0.1 的目标平台：原生窗口、菜单、对话框、NSPanel 弹出层、IME、VoiceOver 投影与实时性能验收。',
      windows:
        '目前可通过 Winit + WGPU 编译。原生运行时、视觉与无障碍验收是 0.3 里程碑。',
      linux:
        '目前可通过 Winit + WGPU 编译。原生运行时、视觉与无障碍验收是 0.3 里程碑。',
    },
    roadmapTitle: '路线图',
    shipped: '已发布',
    roadmap: {
      m1: {
        label: 'macOS 优先的地基',
        detail: '2026-08-27 已发布至 crates.io。',
      },
      m2: {
        label: '无样式组件契约',
        detail: '弹出菜单、Select、Combobox、对话框、表格、树——在 macOS 上实机验收。',
      },
      m3: {
        label: 'Windows 与 Linux 对齐',
        detail: '两个平台上的原生视觉、IME、无障碍与性能验收。',
      },
      m4: {
        label: '稳定的跨平台契约',
        detail: '成文的兼容性政策，处处达标的资源预算。',
      },
    },
    statusLink: '完整状态与路线图',
  },
  cta: {
    title: '做点原生的东西。',
    body: '添加 crate、写一个视图、发布签名应用——让空闲的窗口真正空闲。',
    docs: '阅读文档',
    star: '在 GitHub 加星',
  },
  footer: {
    description: '一个损伤驱动、GPU 加速的 Rust 桌面 GUI 框架。',
    project: '项目',
    packages: '包',
    community: '社区',
    links: {
      docs: '文档',
      viewApi: 'View API',
      architecture: '架构',
      status: '状态与路线图',
      changelog: '更新日志',
      crate: 'crates.io 上的 quickgui',
      docsRs: 'docs.rs API 文档',
      solid: 'Solid 2 渲染器',
      cli: 'CLI 与打包',
      github: 'GitHub',
      issues: 'Issues',
      examples: '示例',
      license: '许可证',
    },
    license: 'MIT 或 Apache-2.0，任选其一',
  },
}
