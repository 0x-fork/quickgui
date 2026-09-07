package ui

import (
	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

// CreateSignal returns a reactive getter and setter for component state.
func CreateSignal[T any](value T, options ...reactive.SignalOptions) (reactive.Accessor[T], reactive.Setter[T]) {
	return reactive.CreateSignal(value, options...)
}

func CreateMemo[T any](compute func() T, options ...reactive.SignalOptions) reactive.Accessor[T] {
	return reactive.CreateMemo(compute, options...)
}

func CreateEffect(fn func()) { reactive.CreateEffect(fn) }

func CreateRenderEffect(fn func()) { reactive.CreateRenderEffect(fn) }

func Batch(fn func()) { reactive.Batch(fn) }

func Untrack[T any](fn func() T) T { return reactive.Untrack(fn) }

func Flush() { reactive.Flush() }

func OnCleanup(fn func()) { reactive.OnCleanup(fn) }

// NativeElement applies ordinary children, styles, options, and reactive ownership
// to a native node kind supplied by an extension package.
func NativeElement(tag uint8, arguments ...any) *native.Node {
	node := native.CreateElement(tag)
	applyArguments(node, arguments)
	return node
}

// View declares children followed by style records and property options.
// Multiple styles merge in order; callback children run once when mounted.
func View(arguments ...any) *native.Node {
	node := native.CreateElement(protocol.TagView)
	applyArguments(node, arguments)
	return node
}

func Text(arguments ...any) *native.Node {
	node := native.CreateElement(protocol.TagView)
	applyArguments(node, arguments)
	return node
}

func Button(arguments ...any) *native.Node {
	node := native.CreateElement(protocol.TagButton)
	applyArguments(node, arguments)
	return node
}

func Input(arguments ...any) *native.Node {
	node := native.CreateElement(protocol.TagInput)
	applyArguments(node, arguments)
	return node
}

func TextArea(arguments ...any) *native.Node {
	return Input(append(arguments, Multiline(true))...)
}

func Markdown(arguments ...any) *native.Node {
	node := native.CreateElement(protocol.TagMarkdown)
	applyArguments(node, arguments)
	return node
}

func Image(arguments ...any) *native.Node {
	node := native.CreateElement(protocol.TagImage)
	applyArguments(node, arguments)
	return node
}

// SVG renders inline SVG markup supplied through Value using the Rust renderer.
func SVG(arguments ...any) *native.Node {
	node := native.CreateElement(protocol.TagSvg)
	applyArguments(node, arguments)
	return node
}

// Shader paints WGSL supplied through Value, with up to sixteen parameter floats.
func Shader(arguments ...any) *native.Node {
	node := native.CreateElement(protocol.TagShader)
	applyArguments(node, arguments)
	return node
}

// VirtualList lays out and paints the visible children using the core's virtual list.
func VirtualList(arguments ...any) *native.Node {
	node := native.CreateElement(protocol.TagVirtualList)
	applyArguments(node, arguments)
	return node
}
