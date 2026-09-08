package ui

import (
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
func NativeElement(tag uint8, arguments ...any) *Element {
	return newElement(tag, arguments)
}

// View constructs a retained container from children. Configure it with fluent
// properties, events, and Styles. Legacy constructor options remain supported.
func View(children ...any) *Element {
	return newElement(protocol.TagView, children)
}

func Text(children ...any) *Element {
	return newElement(protocol.TagView, children)
}

func Button(children ...any) *Element {
	return newElement(protocol.TagButton, children)
}

func Input(children ...any) *Element {
	return newElement(protocol.TagInput, children)
}

func TextArea(children ...any) *Element {
	return Input(children...).Multiline(true)
}

func Markdown(children ...any) *Element {
	return newElement(protocol.TagMarkdown, children)
}

func Image(children ...any) *Element {
	return newElement(protocol.TagImage, children)
}

// SVG renders inline SVG markup supplied through Value using the Rust renderer.
func SVG(children ...any) *Element {
	return newElement(protocol.TagSvg, children)
}

// Shader paints WGSL supplied through Value, with up to sixteen parameter floats.
func Shader(children ...any) *Element {
	return newElement(protocol.TagShader, children)
}

// VirtualList lays out and paints the visible children using the core's virtual list.
func VirtualList(children ...any) *Element {
	return newElement(protocol.TagVirtualList, children)
}
