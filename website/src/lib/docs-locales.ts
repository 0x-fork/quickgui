import type { Locale } from '../i18n'
import type { ComponentDoc } from './component-docs'
import type {
  DocsOutlineItem,
  DocsPageMeta,
  DocsSlug,
} from './docs'

type TranslatedLocale = Exclude<Locale, 'en'>

interface GuideTranslation {
  title: string
  description: string
  outline: readonly DocsOutlineItem[]
  searchTerms: readonly string[]
}

const GUIDE_TRANSLATIONS: Record<
  TranslatedLocale,
  Record<DocsSlug, GuideTranslation>
> = {
  zh: {
    'getting-started': {
      title: '入门',
      description: '使用 QuickGUI UI 创建并运行原生 QuickGUI 应用。',
      outline: [
        { id: '系统要求', title: '系统要求' },
        { id: '创建项目', title: '创建项目' },
        { id: '第一个窗口', title: '第一个窗口' },
        { id: '运行与构建', title: '运行与构建' },
        { id: '后续步骤', title: '后续步骤' },
      ],
      searchTerms: ['安装', '创建', '命令行', '窗口', 'Bun', 'macOS'],
    },
    'project-structure': {
      title: '项目结构',
      description: '了解生成的 QuickGUI UI 项目中各个文件的用途。',
      outline: [
        { id: '生成的文件', title: '生成的文件' },
        { id: '应用入口', title: '应用入口' },
        { id: '配置', title: '配置' },
        { id: '软件包', title: '软件包' },
      ],
      searchTerms: ['文件', '配置', '入口', '软件包', 'quickgui.config'],
    },
    ui: {
      title: '使用 QuickGUI UI',
      description: '使用 QuickGUI UI 响应式能力渲染保留式原生界面。',
      outline: [
        { id: '渲染模型', title: '渲染模型' },
        { id: '响应式状态', title: '响应式状态' },
        { id: '窗口生命周期', title: '窗口生命周期' },
        { id: '当前窗口', title: '当前窗口' },
        { id: '路由', title: '路由' },
      ],
      searchTerms: ['Go', '信号', '组件', '响应式', '生命周期', '路由'],
    },
    styling: {
      title: '样式与布局',
      description: '使用熟悉的属性布局并设置原生节点样式。',
      outline: [
        { id: 'style-属性', title: 'style 属性' },
        { id: '合并样式', title: '合并样式' },
        { id: 'flexbox', title: 'Flexbox' },
        { id: '网格布局', title: '网格布局' },
        { id: '文本与颜色', title: '文本与颜色' },
        { id: '交互状态', title: '交互状态' },
        { id: '分组与具名分组悬停', title: '分组与具名分组悬停' },
      ],
      searchTerms: ['样式', '布局', 'Flexbox', '网格', '颜色', '悬停'],
    },
    animations: {
      title: '过渡与动画',
      description: '在保留式 Go 组件中为原生样式变化和图像添加动画。',
      outline: [
        { id: '悬停过渡', title: '悬停过渡' },
        { id: '状态驱动的动画', title: '状态驱动的动画' },
        { id: '时长与帧率', title: '时长与帧率' },
        { id: '支持的属性', title: '支持的属性' },
        { id: '挂载与减少动态效果', title: '挂载与减少动态效果' },
        { id: '动态图像', title: '动态图像' },
      ],
      searchTerms: ['过渡', '动画', '缓动', '时长', '悬停', '不透明度', '减少动态效果', 'GIF', 'WebP'],
    },
    components: {
      title: '组件',
      description: '在基础组件与具备无障碍行为的复合组件之间进行选择。',
      outline: [
        { id: '基础组件', title: '基础组件' },
        { id: '复合组件', title: '复合组件' },
        { id: '受控状态', title: '受控状态' },
        { id: '组件类别', title: '组件类别' },
        { id: '设置各部件的样式', title: '设置各部件的样式' },
      ],
      searchTerms: ['基础组件', '复合组件', '按钮', '标签页', '复选框', '无样式'],
    },
    'forms-and-input': {
      title: '表单与输入',
      description: '构建受控字段、选项和选择控件。',
      outline: [
        { id: '文本输入', title: '文本输入' },
        { id: '字段', title: '字段' },
        { id: '选项', title: '选项' },
        { id: '选择器', title: '选择器' },
        { id: '事件', title: '事件' },
      ],
      searchTerms: ['输入', '字段', '复选框', '单选框', '选择器', '事件'],
    },
    'overlays-and-dialogs': {
      title: '浮层与对话框',
      description: '显示窗口内浮层和操作系统原生对话框。',
      outline: [
        { id: 'popover', title: 'Popover' },
        { id: 'dialog', title: 'Dialog' },
        { id: '系统浮窗', title: '系统浮窗' },
        { id: '原生对话框', title: '原生对话框' },
      ],
      searchTerms: ['浮层', '对话框', '弹窗', '警告', '文件选择器'],
    },
    'swift-ui': {
      title: 'SwiftUI',
      description: '在 QuickGUI UI 应用中挂载真正的 SwiftUI 控件。',
      outline: [
        { id: 'host', title: 'Host' },
        { id: '原生控件', title: '原生控件' },
        { id: '受控值', title: '受控值' },
        { id: '尺寸', title: '尺寸' },
        { id: '平台支持', title: '平台支持' },
      ],
      searchTerms: ['SwiftUI', '托管', '滑块', '开关', '选择器', '原生'],
    },
    'swift-ui-hosting': {
      title: '修饰器与托管',
      description: '设置 SwiftUI 控件样式，并在其中反向托管 QuickGUI 内容。',
      outline: [
        { id: '修饰器', title: '修饰器' },
        { id: 'liquid-glass', title: 'Liquid Glass' },
        { id: '反向托管', title: '反向托管' },
        { id: 'swiftui-popover', title: 'SwiftUI Popover' },
      ],
      searchTerms: ['修饰器', '玻璃效果', 'QuickGUIHostView', '浮窗', '反向托管'],
    },
  },
  ja: {
    'getting-started': {
      title: 'はじめに',
      description: 'QuickGUI UI を使ってネイティブ QuickGUI アプリを作成し、実行します。',
      outline: [
        { id: '動作要件', title: '動作要件' },
        { id: 'プロジェクトを作成', title: 'プロジェクトを作成' },
        { id: '最初のウィンドウ', title: '最初のウィンドウ' },
        { id: '実行とビルド', title: '実行とビルド' },
        { id: '次のステップ', title: '次のステップ' },
      ],
      searchTerms: ['インストール', '作成', 'CLI', 'ウィンドウ', 'Bun', 'macOS'],
    },
    'project-structure': {
      title: 'プロジェクト構成',
      description: '生成された QuickGUI UI プロジェクトの各ファイルを理解します。',
      outline: [
        { id: '生成されるファイル', title: '生成されるファイル' },
        { id: 'アプリケーションのエントリ', title: 'アプリケーションのエントリ' },
        { id: '設定', title: '設定' },
        { id: 'パッケージ', title: 'パッケージ' },
      ],
      searchTerms: ['ファイル', '設定', 'エントリ', 'パッケージ', 'quickgui.config'],
    },
    ui: {
      title: 'QuickGUI UI の使い方',
      description: 'QuickGUI UI のリアクティビティで保持型のネイティブ UI を描画します。',
      outline: [
        { id: 'レンダリングモデル', title: 'レンダリングモデル' },
        { id: 'リアクティブな状態', title: 'リアクティブな状態' },
        { id: 'ウィンドウのライフサイクル', title: 'ウィンドウのライフサイクル' },
        { id: '現在のウィンドウ', title: '現在のウィンドウ' },
        { id: 'ルーティング', title: 'ルーティング' },
      ],
      searchTerms: ['Go', 'シグナル', 'コンポーネント', 'リアクティブ', 'ライフサイクル', 'ルーター'],
    },
    styling: {
      title: 'スタイルとレイアウト',
      description: '使い慣れたプロパティでネイティブノードを配置し、スタイルします。',
      outline: [
        { id: 'style-プロパティ', title: 'style プロパティ' },
        { id: 'スタイルのマージ', title: 'スタイルのマージ' },
        { id: 'flexbox', title: 'Flexbox' },
        { id: 'グリッド', title: 'グリッド' },
        { id: 'テキストと色', title: 'テキストと色' },
        { id: 'インタラクション状態', title: 'インタラクション状態' },
        { id: 'グループと名前付きグループのホバー', title: 'グループと名前付きグループのホバー' },
      ],
      searchTerms: ['スタイル', 'レイアウト', 'Flexbox', 'グリッド', '色', 'ホバー'],
    },
    animations: {
      title: 'トランジションとアニメーション',
      description: '保持型の Go コンポーネントでネイティブのスタイル変更や画像をアニメーション化します。',
      outline: [
        { id: 'ホバーのトランジション', title: 'ホバーのトランジション' },
        { id: '状態によるアニメーション', title: '状態によるアニメーション' },
        { id: '時間とフレームレート', title: '時間とフレームレート' },
        { id: '対応するプロパティ', title: '対応するプロパティ' },
        { id: 'マウントと視差効果の抑制', title: 'マウントと視差効果の抑制' },
        { id: 'アニメーション画像', title: 'アニメーション画像' },
      ],
      searchTerms: ['トランジション', 'アニメーション', 'イージング', '時間', 'ホバー', '不透明度', '視差効果', 'GIF', 'WebP'],
    },
    components: {
      title: 'コンポーネント',
      description: 'プリミティブと、アクセシブルな複合コンポーネントを使い分けます。',
      outline: [
        { id: 'プリミティブ', title: 'プリミティブ' },
        { id: '複合コンポーネント', title: '複合コンポーネント' },
        { id: '制御された状態', title: '制御された状態' },
        { id: 'コンポーネントの種類', title: 'コンポーネントの種類' },
        { id: '各パーツのスタイル', title: '各パーツのスタイル' },
      ],
      searchTerms: ['プリミティブ', '複合コンポーネント', 'ボタン', 'タブ', 'チェックボックス', 'スタイルなし'],
    },
    'forms-and-input': {
      title: 'フォームと入力',
      description: '制御されたフィールド、選択肢、選択コントロールを作成します。',
      outline: [
        { id: 'テキスト入力', title: 'テキスト入力' },
        { id: 'フィールド', title: 'フィールド' },
        { id: '選択コントロール', title: '選択コントロール' },
        { id: 'select', title: 'Select' },
        { id: 'イベント', title: 'イベント' },
      ],
      searchTerms: ['入力', 'フィールド', 'チェックボックス', 'ラジオ', '選択', 'イベント'],
    },
    'overlays-and-dialogs': {
      title: 'オーバーレイとダイアログ',
      description: 'ウィンドウ内オーバーレイと OS のネイティブダイアログを表示します。',
      outline: [
        { id: 'popover', title: 'Popover' },
        { id: 'dialog', title: 'Dialog' },
        { id: 'システムポップオーバー', title: 'システムポップオーバー' },
        { id: 'ネイティブダイアログ', title: 'ネイティブダイアログ' },
      ],
      searchTerms: ['ポップオーバー', 'ダイアログ', 'オーバーレイ', 'アラート', 'ファイルピッカー'],
    },
    'swift-ui': {
      title: 'SwiftUI',
      description: 'QuickGUI UI アプリ内に本物の SwiftUI コントロールをマウントします。',
      outline: [
        { id: 'host', title: 'Host' },
        { id: 'ネイティブコントロール', title: 'ネイティブコントロール' },
        { id: '制御値', title: '制御値' },
        { id: 'サイズ', title: 'サイズ' },
        { id: '対応プラットフォーム', title: '対応プラットフォーム' },
      ],
      searchTerms: ['SwiftUI', 'ホスト', 'スライダー', 'トグル', 'ピッカー', 'ネイティブ'],
    },
    'swift-ui-hosting': {
      title: 'モディファイアとホスティング',
      description: 'SwiftUI コントロールをスタイルし、その中に QuickGUI の内容をリバースホストします。',
      outline: [
        { id: 'モディファイア', title: 'モディファイア' },
        { id: 'liquid-glass', title: 'Liquid Glass' },
        { id: 'リバースホスティング', title: 'リバースホスティング' },
        { id: 'swiftui-ポップオーバー', title: 'SwiftUI ポップオーバー' },
      ],
      searchTerms: ['モディファイア', 'ガラス', 'QuickGUIHostView', 'ポップオーバー', 'リバースホスト'],
    },
  },
}

const COMPONENT_DESCRIPTION_TRANSLATIONS: Record<
  TranslatedLocale,
  Record<string, string>
> = {
  zh: {
    'ui/view': '用于布局、绘制、指针输入和无障碍支持的通用保留式容器。',
    'ui/text': '使用继承的排版、文本选择和无障碍属性来排版并绘制 Unicode 文本。',
    'ui/button': '可访问的按压目标，可自行设置样式并组合文本或其他内容。',
    'ui/input': '由核心管理的受控文本编辑器，支持原生键盘和文本服务。',
    'ui/text-area': '多行文本编辑基础组件，受控值约定与 Input 相同。',
    'ui/markdown': '以保留模式渲染 Markdown，并支持用于流式内容的增量模式。',
    'ui/image': '使用保留式图像资源显示文件系统路径、文件 URL 或 base64 数据 URL。',
    'ui/svg': '无需加载外部 SVG 资源即可渲染完整的内联 SVG 文档。',
    'ui/shader': '使用有限数量的数值着色器参数绘制经过验证的 WGSL。',
    'ui/virtual-list': '为长列表或不等高集合布局并绘制可见部分。',
    'ui/terminal': '嵌入保留式 Ghostty 终端表面，并报告进程生命周期事件。',
    'ui/checkbox': '二态或不确定状态的选择控件，指示器可独立设置样式。',
    'ui/checkbox-group': '管理一组有限的复选框值，并可派生出可选的父复选框状态。',
    'ui/radio': '单个可选项，通常在 RadioGroup 中声明。',
    'ui/radio-group': '管理单选状态以及 Radio 项之间的键盘移动。',
    'ui/switch': '开关控件，根节点作为轨道，滑块可独立设置样式。',
    'ui/toggle': '可按压的控件，会保留开启或关闭的按下状态。',
    'ui/toggle-group': '配合移动式键盘焦点管理单选或多选的按下值。',
    'ui/slider': '单滑块或多滑块范围输入，指针与键盘交互由核心管理。',
    'ui/number-field': '数字编辑器，提供递增、递减和拖动调节部件。',
    'ui/select': '单选或多选选择器，选项界面显示在原生浮窗中。',
    'ui/combobox': '将可编辑文本、筛选、可选标签项和原生建议界面结合在一起。',
    'ui/autocomplete': '筛选自由输入文本的建议项，无需选中某个值。',
    'ui/field': '将控件与标签、说明、验证状态和错误信息关联起来。',
    'ui/fieldset': '在同一个图例和共享语义状态下组织相关字段。',
    'ui/date-field': '可通过键盘编辑的分段日期字段，日期部分会适配区域设置。',
    'ui/time-field': '可通过键盘编辑的分段时间字段，包含时、分、秒和上下午部分。',
    'ui/calendar': '支持键盘导航的月历网格，用于选择单个日期。',
    'ui/otp-field': '由可分别设置样式的输入格组成、长度受限的一次性验证码编辑器。',
    'ui/tabs': '在带标签的面板之间切换，支持自动或手动键盘激活。',
    'ui/accordion': '管理一个或多个可展开区域及其键盘焦点。',
    'ui/collapsible': '通过触发器显示或隐藏单个面板，同时保留无障碍状态。',
    'ui/splitter': '创建可调整大小的面板，分隔手柄支持指针和键盘操作。',
    'ui/scroll-area': '组合可滚动视口，以及可由调用方设置样式的滚动条、滑块和角落。',
    'ui/table': '虚拟化表格集合，包含表头、行和单元格部件。',
    'ui/tree': '虚拟化层级集合，支持展开、选择和键盘导航。',
    'ui/separator': '具备语义的水平或垂直分隔线，不附带视觉样式。',
    'ui/avatar': '显示图像；图像不可用时显示后备内容。',
    'ui/progress': '表示确定或不确定的任务进度，可组合标签和轨道。',
    'ui/meter': '显示已知范围内的标量测量值，并带有语义化的低值、高值和最佳区间。',
    'ui/toolbar': '在一套移动焦点的键盘模型下组织操作和输入。',
    'ui/popover': '在当前窗口中，将调用方设置样式的内容放在锚点旁。',
    'ui/system-popover': '在独立的原生子窗口中显示 QuickGUI 内容，可超出所属窗口边界。',
    'ui/dialog': '窗口内模态框，支持焦点约束、关闭操作和精确的过渡结束时机。',
    'ui/alert-dialog': '专注确认操作的对话框变体，默认采用更严格的背景点击关闭策略。',
    'ui/tooltip': '在核心精确控制的延迟后显示辅助信息，并支持 Provider 级预热。',
    'ui/preview-card': '在触发器旁显示更丰富的悬停或聚焦预览，不改变当前页面。',
    'ui/toast': '显示定时堆叠通知，支持操作、关闭和滑动手势。',
    'ui/menu': '可完全组合的菜单，支持子菜单、分组、链接、复选项和单选项。',
    'ui/popover-menu': '在原生浮窗中渲染的紧凑型模型驱动菜单。',
    'ui/context-menu': '在触发器上按下次要按钮时打开模型驱动的原生菜单。',
    'ui/menubar': '管理窗口内的水平菜单栏及其菜单项。',
    'ui/navigation-menu': '披露式导航界面，包含带方向动画的视口和弹出部件。',
    'ui/router': '将 QuickGUI 核心路由投影到 QuickGUI UI，支持嵌套布局和容量受限的内存历史。',
    'swift-ui/host': '基于 NSHostingView 的叶节点，用于在 QuickGUI 中挂载一棵 SwiftUI 组件树。',
    'swift-ui/button': '原生 SwiftUI 按钮，支持文本、SF Symbol、语义角色和按下处理。',
    'swift-ui/slider': '受控的原生滑块，支持连续值或步进值。',
    'swift-ui/toggle': '受控的原生 SwiftUI 开关。',
    'swift-ui/progress-view': '确定或不确定的原生 SwiftUI 进度指示器。',
    'swift-ui/stepper': '受控的原生数字步进器，数值范围可限制。',
    'swift-ui/text-field': '受控的原生 SwiftUI 文本字段，提供编辑和提交回调。',
    'swift-ui/secure-field': '隐藏输入内容的原生编辑器，受控约定与 TextField 相同。',
    'swift-ui/picker': '受控的 SwiftUI 选择器，支持菜单、分段、单选组和内联样式。',
    'swift-ui/segmented-control': '原生分段选择器，支持值选择和中性的 Xcode 风格标签页角色。',
    'swift-ui/date-picker': '受控的原生日期或日期时间选择器。',
    'swift-ui/color-picker': '原生 SwiftUI 颜色选择器，返回 CSS 风格的 RGBA 颜色字符串。',
    'swift-ui/gauge': '原生 SwiftUI 仪表，支持线性和圆形附属样式。',
    'swift-ui/quickgui-host-view': '在 SwiftUI 层级中反向托管一棵普通的 QuickGUI 子树。',
    'swift-ui/popover': '从组合式 SwiftUI 触发器显示原生 SwiftUI 浮窗。',
  },
  ja: {
    'ui/view': 'レイアウト、描画、ポインター入力、アクセシビリティに使える汎用の保持型コンテナです。',
    'ui/text': '継承されたタイポグラフィ、選択、アクセシビリティを使って Unicode テキストを組版・描画します。',
    'ui/button': 'テキストなどの内容を自由に組み合わせてスタイルできる、アクセシブルな押下ターゲットです。',
    'ui/input': 'ネイティブのキーボードとテキストサービスに対応した、コアが管理する制御テキストエディターです。',
    'ui/text-area': 'Input と同じ制御値の規約を持つ、複数行テキスト編集プリミティブです。',
    'ui/markdown': '保持型の Markdown を描画し、ストリーミング内容向けの差分更新にも対応します。',
    'ui/image': 'ファイルパス、ファイル URL、base64 データ URL を保持型の画像リソースとして表示します。',
    'ui/svg': '外部リソースを読み込まずに、完全なインライン SVG ドキュメントを描画します。',
    'ui/shader': '検証済みの WGSL を、数を制限した数値シェーダーパラメーターで描画します。',
    'ui/virtual-list': '長いリストや高さが可変のコレクションの表示範囲をレイアウトして描画します。',
    'ui/terminal': '保持型の Ghostty ターミナルサーフェスを埋め込み、プロセスのライフサイクルイベントを通知します。',
    'ui/checkbox': '独立してスタイルできるインジケーターを持つ、二値または不定状態の選択コントロールです。',
    'ui/checkbox-group': '有限個のチェックボックス値をまとめ、必要に応じて親チェックボックスの状態も導出します。',
    'ui/radio': '選択可能な 1 項目です。通常は RadioGroup 内で宣言します。',
    'ui/radio-group': 'Radio 項目間の単一選択とキーボード移動を管理します。',
    'ui/switch': 'ルートのトラックと個別にスタイルできるつまみを持つ、オン／オフコントロールです。',
    'ui/toggle': 'オン／オフの押下状態を保持する、押下可能なコントロールです。',
    'ui/toggle-group': 'ロービングフォーカスを使い、単一または複数の押下値を管理します。',
    'ui/slider': 'ポインターとキーボード操作をコアが管理する、単一または複数つまみの範囲入力です。',
    'ui/number-field': '増加、減少、ドラッグ調整用のパーツを備えた数値エディターです。',
    'ui/select': 'ネイティブのポップオーバーに選択肢を表示する、単一または複数選択のピッカーです。',
    'ui/combobox': '編集可能なテキスト、絞り込み、任意のチップ、ネイティブの候補表示を組み合わせます。',
    'ui/autocomplete': '選択値を必須にせず、自由入力テキストの候補を絞り込みます。',
    'ui/field': '1 つのコントロールをラベル、説明、検証状態、エラーメッセージに関連付けます。',
    'ui/fieldset': '関連するフィールドを 1 つの凡例と共有セマンティック状態の下にまとめます。',
    'ui/date-field': 'ロケールに応じた日付パーツを持つ、キーボード編集可能な分割日付フィールドです。',
    'ui/time-field': '時、分、秒、午前／午後の各パーツを持つ、キーボード編集可能な分割時刻フィールドです。',
    'ui/calendar': '1 つの日付を選ぶための、キーボードで操作できる月間カレンダーグリッドです。',
    'ui/otp-field': '個別にスタイルできる入力セルで構成された、長さ制限付きのワンタイムコードエディターです。',
    'ui/tabs': 'ラベル付きパネルを切り替え、キーボードによる自動または手動のアクティブ化に対応します。',
    'ui/accordion': '1 つ以上の展開セクションと、そのキーボードフォーカスを管理します。',
    'ui/collapsible': 'アクセシブルな状態を保ちながら、トリガーで 1 つのパネルを表示／非表示にします。',
    'ui/splitter': 'ポインターとキーボードで操作できるハンドルを備えた、サイズ変更可能なペインを作ります。',
    'ui/scroll-area': 'スクロール可能なビューポートと、呼び出し側でスタイルできるスクロールバー、つまみ、コーナーを組み合わせます。',
    'ui/table': 'ヘッダー、行、セルのパーツを持つ仮想化テーブルコレクションです。',
    'ui/tree': '展開、選択、キーボードナビゲーションに対応した仮想化階層コレクションです。',
    'ui/separator': '組み込みの見た目を持たない、セマンティックな水平または垂直区切り線です。',
    'ui/avatar': '画像を表示し、読み込めない場合はフォールバックを表示します。',
    'ui/progress': '確定または不確定のタスク進捗を、組み合わせ可能なラベルとトラックで表します。',
    'ui/meter': '既知の範囲内のスカラー測定値を、低・高・最適のセマンティックな帯域とともに表示します。',
    'ui/toolbar': '1 つのロービングフォーカスモデルの下に操作と入力をまとめます。',
    'ui/popover': '呼び出し側がスタイルした内容を、現在のウィンドウ内でアンカーの横に配置します。',
    'ui/system-popover': 'QuickGUI の内容を独立したネイティブ子ウィンドウに表示し、親ウィンドウの外側にも広げられます。',
    'ui/dialog': 'フォーカスの閉じ込め、閉じる操作、正確なトランジション完了を備えたウィンドウ内モーダルです。',
    'ui/alert-dialog': '確認操作に特化し、背景クリックで閉じる既定動作をより厳格にしたダイアログです。',
    'ui/tooltip': 'コアが正確に管理する遅延後に補助情報を表示し、Provider 単位のウォームアップにも対応します。',
    'ui/preview-card': '現在の画面を変えず、トリガーの横により詳しいホバー／フォーカスプレビューを表示します。',
    'ui/toast': '操作、閉じる、スワイプに対応した、時間制御される積み重ね通知を表示します。',
    'ui/menu': 'サブメニュー、グループ、リンク、チェック項目、ラジオ項目を組み合わせられるメニューです。',
    'ui/popover-menu': 'ネイティブのポップオーバー内に描画される、コンパクトなモデル駆動メニューです。',
    'ui/context-menu': 'トリガーを副ボタンで押すと、モデル駆動のネイティブメニューを開きます。',
    'ui/menubar': 'ウィンドウ内の水平メニューバーとその項目を管理します。',
    'ui/navigation-menu': '方向付きアニメーション、ビューポート、ポップアップの各パーツを備えた開閉式ナビゲーションです。',
    'ui/router': 'QuickGUI コアのルーティングを QuickGUI UI に投影し、ネストされたレイアウトと上限付きメモリ履歴を提供します。',
    'swift-ui/host': 'NSHostingView を基盤とし、QuickGUI 内に 1 つの SwiftUI コンポーネントツリーをマウントするリーフです。',
    'swift-ui/button': 'テキスト、SF Symbol、セマンティックロール、押下処理に対応したネイティブ SwiftUI Button です。',
    'swift-ui/slider': '連続値または段階値に対応した、制御されたネイティブ Slider です。',
    'swift-ui/toggle': '制御されたネイティブ SwiftUI オン／オフスイッチです。',
    'swift-ui/progress-view': '確定または不確定のネイティブ SwiftUI 進捗インジケーターです。',
    'swift-ui/stepper': '値の範囲を制限できる、制御されたネイティブ数値 Stepper です。',
    'swift-ui/text-field': '編集と送信のコールバックを持つ、制御されたネイティブ SwiftUI TextField です。',
    'swift-ui/secure-field': 'TextField と同じ制御規約を持つ、入力内容を隠すネイティブエディターです。',
    'swift-ui/picker': 'メニュー、セグメント、ラジオグループ、インラインの各スタイルに対応した、制御された SwiftUI Picker です。',
    'swift-ui/segmented-control': '値選択とニュートラルな Xcode 風タブロールを持つ、ネイティブのセグメントピッカーです。',
    'swift-ui/date-picker': '制御されたネイティブの日付または日時ピッカーです。',
    'swift-ui/color-picker': 'CSS 形式の RGBA 文字列を返す、ネイティブ SwiftUI カラーピッカーです。',
    'swift-ui/gauge': '線形と円形のアクセサリースタイルに対応した、ネイティブ SwiftUI Gauge です。',
    'swift-ui/quickgui-host-view': 'SwiftUI 階層の中へ通常の QuickGUI サブツリーをリバースホストします。',
    'swift-ui/popover': '組み合わせた SwiftUI トリガーから、ネイティブ SwiftUI Popover を表示します。',
  },
}

export const COMPONENT_DOC_LABELS = {
  en: {
    import: 'Import',
    usage: 'Usage',
    anatomy: 'Anatomy',
    keyProps: 'Key props',
    prop: 'Prop',
    purpose: 'Purpose',
  },
  zh: {
    import: '导入',
    usage: '用法',
    anatomy: '结构',
    keyProps: '主要属性',
    prop: '属性',
    purpose: '说明',
  },
  ja: {
    import: 'インポート',
    usage: '使い方',
    anatomy: '構成',
    keyProps: '主なプロパティ',
    prop: 'プロパティ',
    purpose: '説明',
  },
} as const satisfies Record<Locale, Record<string, string>>

export function localizedDocsPage(
  page: DocsPageMeta,
  locale: Locale,
): DocsPageMeta {
  if (locale === 'en') return page
  return { ...page, ...GUIDE_TRANSLATIONS[locale][page.slug] }
}

export function localizedComponentDescription(
  component: ComponentDoc,
  locale: Locale,
): string {
  if (locale === 'en') return component.description
  return (
    COMPONENT_DESCRIPTION_TRANSLATIONS[locale][
      `${component.kind}/${component.slug}`
    ] ?? component.description
  )
}

export function localizedComponentOutline(
  component: ComponentDoc,
  locale: Locale,
): readonly DocsOutlineItem[] {
  const labels = COMPONENT_DOC_LABELS[locale]
  return [
    { id: labels.import.toLowerCase(), title: labels.import },
    { id: labels.usage.toLowerCase(), title: labels.usage },
    ...(component.parts.length
      ? [{ id: labels.anatomy.toLowerCase(), title: labels.anatomy } as const]
      : []),
    { id: labels.keyProps.toLowerCase().replace(/\s+/g, '-'), title: labels.keyProps },
  ]
}

export function componentDescriptionTranslations(
  locale: TranslatedLocale,
): Readonly<Record<string, string>> {
  return COMPONENT_DESCRIPTION_TRANSLATIONS[locale]
}
