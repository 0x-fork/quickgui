import AppKit
@preconcurrency import SwiftUI

typealias QuickGUIActionCallback = @convention(c) (
  UnsafeMutableRawPointer?,
  UInt64
) -> Void

typealias QuickGUIPresentationCallback = @convention(c) (
  UnsafeMutableRawPointer?,
  UInt64,
  Bool
) -> Void

private let quickGUIGlassEffectInset: CGFloat = 24

private struct QuickGUIElement: Decodable, Identifiable, Equatable {
  let id: UInt64
  let type: String
  let label: String?
  let systemImage: String?
  let role: String?
  let target: String?
  let testID: String?
  let modifiers: [QuickGUIModifier]?
  let hasAction: Bool?
  let matchHorizontal: Bool?
  let matchVertical: Bool?
  let width: Double?
  let height: Double?
  let isPresented: Bool?
  let attachmentAnchor: String?
  let arrowEdge: String?
  let trigger: [QuickGUIElement]?
  let content: [QuickGUIElement]?
}

private struct QuickGUIModifier: Decodable, Equatable {
  let type: String
  let style: String?
  let size: String?
  let shape: String?
  let cornerRadius: Double?
  let color: String?
  let disabled: Bool?

  private enum CodingKeys: String, CodingKey {
    case type = "$type"
    case style
    case size
    case shape
    case cornerRadius
    case color
    case disabled
  }
}

private final class QuickGUIActionSink {
  let context: UnsafeMutableRawPointer?
  let actionCallback: QuickGUIActionCallback?
  let presentationCallback: QuickGUIPresentationCallback?

  init(
    context: UnsafeMutableRawPointer?,
    actionCallback: QuickGUIActionCallback?,
    presentationCallback: QuickGUIPresentationCallback?
  ) {
    self.context = context
    self.actionCallback = actionCallback
    self.presentationCallback = presentationCallback
  }

  func sendAction(_ id: UInt64) {
    actionCallback?(context, id)
  }

  func sendPresentation(_ id: UInt64, _ isPresented: Bool) {
    presentationCallback?(context, id, isPresented)
  }
}

private struct QuickGUIEmbeddedEntry {
  let view: NSView
  let size: NSSize
}

private final class QuickGUIElementStore: ObservableObject {
  @Published var elements: [QuickGUIElement] = []
  @Published private(set) var embeddedRevision: UInt64 = 0
  private var embedded: [UInt64: QuickGUIEmbeddedEntry] = [:]

  func updateElements(_ elements: [QuickGUIElement]) {
    self.elements = elements
  }

  func setEmbeddedView(_ id: UInt64, view: NSView, size: NSSize) {
    let normalized = NSSize(width: max(1, size.width), height: max(1, size.height))
    if let previous = embedded[id], previous.view === view, previous.size == normalized {
      return
    }
    embedded[id] = QuickGUIEmbeddedEntry(view: view, size: normalized)
    embeddedRevision &+= 1
  }

  func removeEmbeddedView(_ id: UInt64) {
    guard embedded.removeValue(forKey: id) != nil else { return }
    embeddedRevision &+= 1
  }

  func embeddedView(_ id: UInt64) -> QuickGUIEmbeddedEntry? {
    embedded[id]
  }

}

private final class QuickGUIEmbeddedContainerView: NSView {
  private var hostedView: NSView?
  private var contentSize = NSSize(width: 1, height: 1)

  override var isFlipped: Bool { true }

  override var intrinsicContentSize: NSSize {
    contentSize
  }

  func update(entry: QuickGUIEmbeddedEntry?) {
    let nextView = entry?.view
    if hostedView !== nextView {
      hostedView?.removeFromSuperview()
      hostedView = nextView
      if let nextView {
        nextView.autoresizingMask = [.width, .height]
        addSubview(nextView)
      }
    }
    if let entry, contentSize != entry.size {
      contentSize = entry.size
      invalidateIntrinsicContentSize()
    }
    needsLayout = true
  }

  override func layout() {
    super.layout()
    hostedView?.frame = bounds
  }

  func detach() {
    hostedView?.removeFromSuperview()
    hostedView = nil
  }
}

private struct QuickGUIEmbeddedRepresentable: NSViewRepresentable {
  @ObservedObject var store: QuickGUIElementStore
  let id: UInt64

  func makeNSView(context: Context) -> QuickGUIEmbeddedContainerView {
    let view = QuickGUIEmbeddedContainerView(frame: .zero)
    view.setAccessibilityElement(false)
    view.update(entry: store.embeddedView(id))
    return view
  }

  func updateNSView(_ nsView: QuickGUIEmbeddedContainerView, context: Context) {
    _ = store.embeddedRevision
    nsView.update(entry: store.embeddedView(id))
  }

  static func dismantleNSView(_ nsView: QuickGUIEmbeddedContainerView, coordinator: ()) {
    nsView.detach()
  }
}

private final class QuickGUIPopoverAnchorView: NSView {
  var didMoveToWindow: (() -> Void)?

  override func viewDidMoveToWindow() {
    super.viewDidMoveToWindow()
    didMoveToWindow?()
  }

  override func hitTest(_ point: NSPoint) -> NSView? {
    nil
  }
}

private struct QuickGUIPopoverAnchorRepresentable: NSViewRepresentable {
  let id: UInt64
  let isPresented: Bool
  let attachmentAnchor: String?
  let arrowEdge: String?
  let content: AnyView
  let actionSink: QuickGUIActionSink

  func makeCoordinator() -> Coordinator {
    Coordinator(id: id, actionSink: actionSink)
  }

  func makeNSView(context: Context) -> QuickGUIPopoverAnchorView {
    let view = QuickGUIPopoverAnchorView(frame: .zero)
    view.setAccessibilityElement(false)
    view.didMoveToWindow = { [weak coordinator = context.coordinator] in
      coordinator?.reconcile()
    }
    context.coordinator.anchor = view
    return view
  }

  func updateNSView(_ nsView: QuickGUIPopoverAnchorView, context: Context) {
    context.coordinator.update(
      anchor: nsView,
      isPresented: isPresented,
      attachmentAnchor: attachmentAnchor,
      arrowEdge: arrowEdge,
      content: content
    )
  }

  static func dismantleNSView(_ nsView: QuickGUIPopoverAnchorView, coordinator: Coordinator) {
    nsView.didMoveToWindow = nil
    coordinator.detach()
  }

  final class Coordinator: NSObject, NSPopoverDelegate {
    fileprivate weak var anchor: QuickGUIPopoverAnchorView?
    private let id: UInt64
    private let actionSink: QuickGUIActionSink
    private var expectedPresented = false
    private var attachmentAnchor: String?
    private var arrowEdge: String?
    private var content = AnyView(EmptyView())
    private var popover: NSPopover?
    private var contentController: NSHostingController<AnyView>?

    init(id: UInt64, actionSink: QuickGUIActionSink) {
      self.id = id
      self.actionSink = actionSink
    }

    fileprivate func update(
      anchor: QuickGUIPopoverAnchorView,
      isPresented: Bool,
      attachmentAnchor: String?,
      arrowEdge: String?,
      content: AnyView
    ) {
      self.anchor = anchor
      self.expectedPresented = isPresented
      self.attachmentAnchor = attachmentAnchor
      self.arrowEdge = arrowEdge
      self.content = content
      reconcile()
    }

    fileprivate func reconcile() {
      guard expectedPresented else {
        popover?.close()
        return
      }
      guard let anchor, anchor.window != nil else { return }

      let controller: NSHostingController<AnyView>
      if let contentController {
        contentController.rootView = content
        controller = contentController
      } else {
        controller = NSHostingController(rootView: content)
        if #available(macOS 13.0, *) {
          controller.sizingOptions = [.preferredContentSize]
        }
        contentController = controller
      }

      let popover: NSPopover
      if let current = self.popover {
        popover = current
      } else {
        popover = NSPopover()
        popover.behavior = .transient
        popover.animates = true
        popover.delegate = self
        self.popover = popover
      }
      popover.contentViewController = controller
      controller.view.layoutSubtreeIfNeeded()
      let fittingSize = controller.view.fittingSize
      if fittingSize.width.isFinite, fittingSize.height.isFinite,
        fittingSize.width > 0, fittingSize.height > 0
      {
        popover.contentSize = fittingSize
      }
      guard !popover.isShown else { return }
      popover.show(
        relativeTo: quickGUIPopoverAnchorRect(attachmentAnchor, in: anchor.bounds),
        of: anchor,
        preferredEdge: quickGUIPopoverPreferredEdge(arrowEdge)
      )
    }

    fileprivate func detach() {
      expectedPresented = false
      popover?.delegate = nil
      popover?.close()
      popover = nil
      contentController = nil
      anchor = nil
    }

    func popoverDidClose(_ notification: Notification) {
      guard expectedPresented else { return }
      expectedPresented = false
      actionSink.sendPresentation(id, false)
    }
  }
}

private struct QuickGUIElementGroup: View {
  let elements: [QuickGUIElement]
  @ObservedObject var store: QuickGUIElementStore
  let actionSink: QuickGUIActionSink

  var body: some View {
    VStack(spacing: 8) {
      ForEach(elements) { element in
        QuickGUIElementView(
          element: element,
          store: store,
          actionSink: actionSink
        )
      }
    }
  }
}

private struct QuickGUIElementView: View {
  let element: QuickGUIElement
  @ObservedObject var store: QuickGUIElementStore
  let actionSink: QuickGUIActionSink

  var body: some View {
    content
  }

  private var content: AnyView {
    switch element.type {
    case "button":
      return swiftUIButton(element)
    case "quickGuiHost":
      var result = AnyView(QuickGUIEmbeddedRepresentable(store: store, id: element.id))
      if element.width != nil || element.height != nil {
        let width = element.width.map { CGFloat($0) }
        let height = element.height.map { CGFloat($0) }
        result = AnyView(
          result.frame(
            width: width,
            height: height
          )
        )
      }
      if let testID = element.testID, !testID.isEmpty {
        result = AnyView(result.accessibilityIdentifier(testID))
      }
      return result
    case "popover":
      return swiftUIPopover(element)
    default:
      return AnyView(EmptyView())
    }
  }

  private func swiftUIButton(_ element: QuickGUIElement) -> AnyView {
    let role: ButtonRole? = switch element.role {
    case "cancel": .cancel
    case "destructive": .destructive
    default: nil
    }
    let label = element.label ?? ""
    let button = Button(role: role) {
      // Always cross the native boundary for an actual SwiftUI Button activation. Rust/JS owns
      // listener presence and drops an event whose target no longer has an onPress handler. This
      // also avoids making a retained SwiftUI closure stale when listener props change in place.
      actionSink.sendAction(element.id)
    } label: {
      if element.label != nil, let systemImage = element.systemImage, !systemImage.isEmpty {
        Label(label, systemImage: systemImage)
      } else {
        Text(label)
      }
    }

    var result = AnyView(button)
    for modifier in element.modifiers ?? [] {
      result = apply(modifier, to: result)
    }
    if let testID = element.testID, !testID.isEmpty {
      result = AnyView(result.accessibilityIdentifier(testID))
    }
    return result
  }

  private func swiftUIPopover(_ element: QuickGUIElement) -> AnyView {
    let trigger = QuickGUIElementGroup(
      elements: element.trigger ?? [],
      store: store,
      actionSink: actionSink
    )
    let popoverContent = QuickGUIElementGroup(
      elements: element.content ?? [],
      store: store,
      actionSink: actionSink
    )
    let anchor = QuickGUIPopoverAnchorRepresentable(
      id: element.id,
      isPresented: element.isPresented ?? false,
      attachmentAnchor: element.attachmentAnchor,
      arrowEdge: element.arrowEdge,
      content: AnyView(popoverContent),
      actionSink: actionSink
    )
    var result = AnyView(
      trigger.background(anchor)
    )
    if let testID = element.testID, !testID.isEmpty {
      result = AnyView(result.accessibilityIdentifier(testID))
    }
    return result
  }

  private func apply(_ modifier: QuickGUIModifier, to view: AnyView) -> AnyView {
    switch modifier.type {
    case "buttonStyle":
      switch modifier.style {
      case "bordered": return AnyView(view.buttonStyle(.bordered))
      case "borderedProminent": return AnyView(view.buttonStyle(.borderedProminent))
      case "borderless": return AnyView(view.buttonStyle(.borderless))
      case "plain": return AnyView(view.buttonStyle(.plain))
      case "glass":
        #if compiler(>=6.2)
          if #available(macOS 26.0, *) {
            return AnyView(view.buttonStyle(.glass))
          }
        #endif
        return AnyView(view.buttonStyle(.bordered))
      case "glassProminent":
        #if compiler(>=6.2)
          if #available(macOS 26.0, *) {
            return AnyView(view.buttonStyle(.glassProminent))
          }
        #endif
        return AnyView(view.buttonStyle(.borderedProminent))
      default: return AnyView(view.buttonStyle(.automatic))
      }
    case "buttonBorderShape":
      switch modifier.shape {
      case "capsule":
        if #available(macOS 14.0, *) {
          return AnyView(view.buttonBorderShape(.capsule))
        }
        return AnyView(view.buttonBorderShape(.roundedRectangle))
      case "roundedRectangle":
        if #available(macOS 14.0, *), let radius = modifier.cornerRadius {
          return AnyView(view.buttonBorderShape(.roundedRectangle(radius: CGFloat(radius))))
        }
        return AnyView(view.buttonBorderShape(.roundedRectangle))
      case "circle":
        if #available(macOS 14.0, *) {
          return AnyView(view.buttonBorderShape(.circle))
        }
        return AnyView(view.buttonBorderShape(.roundedRectangle))
      default: return AnyView(view.buttonBorderShape(.automatic))
      }
    case "controlSize":
      switch modifier.size {
      case "mini": return AnyView(view.controlSize(.mini))
      case "small": return AnyView(view.controlSize(.small))
      case "large": return AnyView(view.controlSize(.large))
      case "extraLarge":
        if #available(macOS 15.0, *) {
          return AnyView(view.controlSize(.extraLarge))
        }
        return AnyView(view.controlSize(.large))
      default: return AnyView(view.controlSize(.regular))
      }
    case "labelStyle":
      switch modifier.style {
      case "iconOnly": return AnyView(view.labelStyle(.iconOnly))
      case "titleAndIcon": return AnyView(view.labelStyle(.titleAndIcon))
      case "titleOnly": return AnyView(view.labelStyle(.titleOnly))
      default: return AnyView(view.labelStyle(.automatic))
      }
    case "tint":
      if let color = modifier.color.flatMap(quickGUIColor) {
        return AnyView(view.tint(color))
      }
      return view
    case "disabled": return AnyView(view.disabled(modifier.disabled ?? true))
    default: return view
    }
  }
}

private func quickGUIPopoverAnchorRect(_ value: String?, in bounds: NSRect) -> NSRect {
  let point: NSPoint
  switch value {
  case "top": point = NSPoint(x: bounds.midX, y: bounds.maxY)
  case "bottom": point = NSPoint(x: bounds.midX, y: bounds.minY)
  case "leading": point = NSPoint(x: bounds.minX, y: bounds.midY)
  case "trailing": point = NSPoint(x: bounds.maxX, y: bounds.midY)
  default: point = NSPoint(x: bounds.midX, y: bounds.midY)
  }
  return NSRect(origin: point, size: NSSize(width: 1, height: 1))
}

private func quickGUIPopoverPreferredEdge(_ value: String?) -> NSRectEdge {
  switch value {
  case "top": return .minY
  case "leading": return .maxX
  case "trailing": return .minX
  default: return .maxY
  }
}

private struct QuickGUIRootView: View {
  @ObservedObject var store: QuickGUIElementStore
  let actionSink: QuickGUIActionSink

  var body: some View {
    QuickGUIElementGroup(
      elements: store.elements,
      store: store,
      actionSink: actionSink
    )
      .padding(quickGUIGlassEffectInset)
      .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .center)
  }
}

private func quickGUIColor(_ value: String) -> Color? {
  switch value.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
  case "primary": return .primary
  case "secondary": return .secondary
  case "red": return .red
  case "orange": return .orange
  case "yellow": return .yellow
  case "green": return .green
  case "blue": return .blue
  case "purple": return .purple
  case "pink": return .pink
  case "white": return .white
  case "gray", "grey": return .gray
  case "black": return .black
  case "clear", "transparent": return .clear
  default: break
  }

  let value = value.trimmingCharacters(in: .whitespacesAndNewlines)
  guard value.hasPrefix("#") else { return nil }
  let hex = String(value.dropFirst())
  let expanded: String
  if hex.count == 3 || hex.count == 4 {
    expanded = hex.map { "\($0)\($0)" }.joined()
  } else {
    expanded = hex
  }
  guard (expanded.count == 6 || expanded.count == 8),
    let number = UInt64(expanded, radix: 16)
  else {
    return nil
  }
  let hasAlpha = expanded.count == 8
  let red = Double((number >> (hasAlpha ? 24 : 16)) & 0xff) / 255
  let green = Double((number >> (hasAlpha ? 16 : 8)) & 0xff) / 255
  let blue = Double((number >> (hasAlpha ? 8 : 0)) & 0xff) / 255
  let alpha = hasAlpha ? Double(number & 0xff) / 255 : 1
  return Color(red: red, green: green, blue: blue, opacity: alpha)
}

private final class QuickGUIHostHandle {
  let actionSink: QuickGUIActionSink
  let store: QuickGUIElementStore
  let view: NSHostingView<QuickGUIRootView>

  init(
    context: UnsafeMutableRawPointer?,
    actionCallback: QuickGUIActionCallback?,
    presentationCallback: QuickGUIPresentationCallback?
  ) {
    let actionSink = QuickGUIActionSink(
      context: context,
      actionCallback: actionCallback,
      presentationCallback: presentationCallback
    )
    let store = QuickGUIElementStore()
    self.actionSink = actionSink
    self.store = store
    self.view = NSHostingView(rootView: QuickGUIRootView(store: store, actionSink: actionSink))
    self.view.sizingOptions = [.intrinsicContentSize]
  }

  func update(json: UnsafePointer<CChar>) -> Bool {
    guard let data = String(cString: json).data(using: .utf8),
      let elements = try? JSONDecoder().decode([QuickGUIElement].self, from: data)
    else {
      return false
    }
    store.updateElements(elements)
    view.invalidateIntrinsicContentSize()
    view.layoutSubtreeIfNeeded()
    return true
  }
}

@_cdecl("quickgui_swift_ui_host_create")
func quickGUISwiftUIHostCreate(
  _ context: UnsafeMutableRawPointer?,
  _ actionCallback: QuickGUIActionCallback?,
  _ presentationCallback: QuickGUIPresentationCallback?
) -> UnsafeMutableRawPointer? {
  precondition(Thread.isMainThread)
  return Unmanaged.passRetained(
    QuickGUIHostHandle(
      context: context,
      actionCallback: actionCallback,
      presentationCallback: presentationCallback
    )
  ).toOpaque()
}

@_cdecl("quickgui_swift_ui_host_view")
func quickGUISwiftUIHostView(
  _ opaqueHandle: UnsafeMutableRawPointer
) -> UnsafeMutableRawPointer {
  precondition(Thread.isMainThread)
  let handle = Unmanaged<QuickGUIHostHandle>.fromOpaque(opaqueHandle).takeUnretainedValue()
  return Unmanaged.passUnretained(handle.view).toOpaque()
}

@_cdecl("quickgui_swift_ui_host_update")
func quickGUISwiftUIHostUpdate(
  _ opaqueHandle: UnsafeMutableRawPointer,
  _ json: UnsafePointer<CChar>
) -> Bool {
  precondition(Thread.isMainThread)
  let handle = Unmanaged<QuickGUIHostHandle>.fromOpaque(opaqueHandle).takeUnretainedValue()
  return handle.update(json: json)
}

@_cdecl("quickgui_swift_ui_host_set_embedded_view")
func quickGUISwiftUIHostSetEmbeddedView(
  _ opaqueHandle: UnsafeMutableRawPointer,
  _ id: UInt64,
  _ opaqueView: UnsafeMutableRawPointer,
  _ width: Double,
  _ height: Double
) -> Bool {
  precondition(Thread.isMainThread)
  guard width.isFinite, height.isFinite, width > 0, height > 0 else { return false }
  let handle = Unmanaged<QuickGUIHostHandle>.fromOpaque(opaqueHandle).takeUnretainedValue()
  let view = Unmanaged<NSView>.fromOpaque(opaqueView).takeUnretainedValue()
  handle.store.setEmbeddedView(
    id,
    view: view,
    size: NSSize(width: width, height: height)
  )
  handle.view.invalidateIntrinsicContentSize()
  return true
}

@_cdecl("quickgui_swift_ui_host_remove_embedded_view")
func quickGUISwiftUIHostRemoveEmbeddedView(
  _ opaqueHandle: UnsafeMutableRawPointer,
  _ id: UInt64
) {
  precondition(Thread.isMainThread)
  let handle = Unmanaged<QuickGUIHostHandle>.fromOpaque(opaqueHandle).takeUnretainedValue()
  handle.store.removeEmbeddedView(id)
  handle.view.invalidateIntrinsicContentSize()
}

@_cdecl("quickgui_swift_ui_host_fitting_size")
func quickGUISwiftUIHostFittingSize(
  _ opaqueHandle: UnsafeMutableRawPointer,
  _ width: UnsafeMutablePointer<Double>,
  _ height: UnsafeMutablePointer<Double>
) {
  precondition(Thread.isMainThread)
  let handle = Unmanaged<QuickGUIHostHandle>.fromOpaque(opaqueHandle).takeUnretainedValue()
  handle.view.layoutSubtreeIfNeeded()
  let size = handle.view.fittingSize
  width.pointee = size.width
  height.pointee = size.height
}

@_cdecl("quickgui_swift_ui_host_release")
func quickGUISwiftUIHostRelease(_ opaqueHandle: UnsafeMutableRawPointer) {
  precondition(Thread.isMainThread)
  Unmanaged<QuickGUIHostHandle>.fromOpaque(opaqueHandle).release()
}
