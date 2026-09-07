import type { en } from './en'

export const zh: typeof en = {
  "meta": {
    "title": "QuickGUI — 用 Go 构建原生桌面应用",
    "description": "用 Go 构建原生桌面应用。快速构建、细粒度响应式、GPU 渲染、熟悉的布局和无障碍组件，无需 CGO。"
  },
  "common": {
    "skipToContent": "跳到正文",
    "getStarted": "开始使用",
    "copy": "复制 “{{text}}”",
    "copied": "已复制",
    "language": "语言"
  },
  "nav": {
    "features": "特性",
    "code": "代码",
    "quickstart": "快速开始",
    "docs": "文档"
  },
  "hero": {
    "badge": "Go · 无需 CGO",
    "titleLine1": "构建原生桌面应用，",
    "titleLine2": "用 Go。",
    "sub": "用普通 Go 代码享受快速构建和细粒度响应式。通过熟悉的布局和无障碍组件，组合 GPU 渲染的原生界面，无需 CGO。"
  },
  "features": {
    "title": "桌面应用需要的, 都在这里",
    "items": {
      "idle": {
        "title": "空闲窗口保持休眠",
        "body": "界面没有变化时，窗口保持休眠。信号只更新相关属性，事件中的更新会合并为一次重绘。"
      },
      "fast": {
        "title": "快速 Go 构建",
        "body": "修改应用时，只需重新编译 Go 代码。原生运行时随应用打包，应用构建禁用 CGO。"
      },
      "layout": {
        "title": "你已熟悉的布局",
        "body": "使用 Flexbox、CSS Grid 和熟悉的样式选项，直接在 Go 中组合布局并复用组件样式。"
      },
      "components": {
        "title": "组件开箱即用",
        "body": "一大批可访问的无样式组件——菜单、对话框、弹出层、select、combobox、标签页、表格、树——随你定制"
      },
      "text": {
        "title": "文本真正好用",
        "body": "选择、编辑、撤销、输入法、emoji、从右到左文字, 表现与所有原生应用一致"
      },
      "a11y": {
        "title": "默认无障碍",
        "body": "屏幕阅读器看到的是真实的按钮、列表和文本, 无需额外代码"
      },
      "lists": {
        "title": "长列表，少量可见节点",
        "body": "虚拟化表格随滚动挂载可见行，轻松管理较大的历史记录和数据视图。"
      },
      "native": {
        "title": "原生桌面能力",
        "body": "真实窗口、系统菜单、文件对话框、托盘图标和通知，都是应用的一部分。"
      },
      "cli": {
        "title": "一个 CLI, 从开发到发布",
        "body": "quickgui dev 边改边跑;quickgui build 直接产出已签名、可安装的发布版本"
      }
    }
  },
  "code": {
    "title": "普通 Go，响应式组件。",
    "lead": "用函数块声明子节点。信号只更新读取它的绑定，组件的其余部分保持挂载。"
  },
  "swiftUi": {
    "title": "在 QuickGUI 中使用原生 SwiftUI",
    "lead": "在 macOS 的 Go 应用中嵌入真正的 SwiftUI 控件，与 QuickGUI 组件一起使用。"
  },
  "quickstart": {
    "title": "从第一个窗口到应用发布。",
    "create": "创建并运行",
    "edit": "格式化与构建",
    "ship": "签名与打包",
    "note": "安装 Go 1.23+ 和 Bun。macOS 还需要 Xcode Command Line Tools。签名和公证使用你自己的 Apple 开发者凭证。"
  },
  "platforms": {
    "available": "现已可用",
    "soon": "开发中"
  },
  "cta": {
    "title": "做点原生的东西",
    "body": "编写 Go 组件，发布自带运行时的原生应用。",
    "docs": "阅读文档",
    "star": "在 GitHub 加星"
  },
  "footer": {
    "license": "MIT 或 Apache-2.0"
  }
}
