import type { en } from './en'

export const zh: typeof en = {
  meta: {
    title: 'QuickGUI — 构建原生桌面应用，告别 WebView',
    description:
      'QuickGUI 是 GPU 加速的 GUI 框架, 用 Rust 或 TypeScript 构建原生桌面应用。熟悉的 Flexbox 与 Grid 布局、真正好用的文本、内置无障碍——应用空闲时零 CPU 占用',
  },
  common: {
    skipToContent: '跳到正文',
    getStarted: '开始使用',
    copy: '复制 “{{text}}”',
    copied: '已复制',
    language: '语言',
  },
  nav: {
    features: '特性',
    code: '代码',
    quickstart: '快速开始',
    docs: '文档',
  },
  hero: {
    badge: 'v{{version}} 已发布至 crates.io',
    titleLine1: '构建原生桌面应用，',
    titleLine2: '告别 WebView',
    sub: '用 Rust 或 TypeScript 构建原生桌面应用的 GPU 加速 GUI 框架。熟悉的布局、真正好用的文本、内置无障碍——应用空闲时零 CPU 占用',
  },
  features: {
    title: '桌面应用需要的, 都在这里',
    items: {
      idle: {
        title: '空闲时零 CPU',
        body: '屏幕上没有变化, 就什么都不运行。空闲窗口不绘制任何帧, 也不耗电',
      },
      fast: {
        title: '默认就快',
        body: '渲染在 GPU 上进行, 并且只重绘窗口中发生变化的部分',
      },
      layout: {
        title: '你已熟悉的布局',
        body: 'Flexbox 与 CSS Grid, 外加 Tailwind 风格的简写。会排网页, 就会排窗口',
      },
      components: {
        title: '组件开箱即用',
        body: '一大批可访问的无样式组件——菜单、对话框、弹出层、select、combobox、标签页、表格、树——随你定制',
      },
      text: {
        title: '文本真正好用',
        body: '选择、编辑、撤销、输入法、emoji、从右到左文字, 表现与所有原生应用一致',
      },
      a11y: {
        title: '默认无障碍',
        body: '屏幕阅读器看到的是真实的按钮、列表和文本, 无需额外代码',
      },
      lists: {
        title: '再大也流畅',
        body: '百万行的表格与列表, 滚动不掉帧',
      },
      native: {
        title: '原生, 而且是真原生',
        body: '真实窗口、原生菜单、对话框、托盘图标与系统通知——不是套壳浏览器',
      },
      cli: {
        title: '一个 CLI, 从开发到发布',
        body: 'quickgui dev 边改边跑;quickgui build 直接产出已签名、可安装的发布版本',
      },
    },
  },
  code: {
    title: '用 Rust 或 TypeScript 来写',
    lead: '想用哪个都行——产出的是同一个原生应用, 而且没有 webview',
  },
  swiftUi: {
    title: '在 QuickGUI 中使用原生 SwiftUI',
    lead: '在 macOS 的 Solid 应用中直接嵌入真正的 SwiftUI 控件。',
  },
  quickstart: {
    title: '一分钟上手',
    rust: {
      addCrate: '添加 crate',
      write: '写一个视图',
      writeBody: '上面的计数器就是完整的 main.rs——直接复制进来',
      run: '运行',
    },
    solid: {
      create: '创建应用',
      ship: '发布',
      note: '你会得到一个真实、已签名、自包含的应用, 连安装包都有',
    },
  },
  platforms: {
    available: '现已可用',
    soon: '开发中',
  },
  cta: {
    title: '做点原生的东西',
    body: '写一个视图, 发布一个真正的应用——让空闲窗口真正空闲',
    docs: '阅读文档',
    star: '在 GitHub 加星',
  },
  footer: {
    license: 'MIT 或 Apache-2.0',
  },
}
