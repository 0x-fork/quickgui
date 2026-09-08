package main

import (
	"encoding/json"
	"log"

	"github.com/egoist/quickgui/go/native"
	"github.com/egoist/quickgui/go/ui"
)

type completion func(string, error)
type systemAction struct {
	label     string
	operation func(completion)
}

type systemState struct {
	window                *native.Window
	status                func(string)
	busy                  func() bool
	setBusy               func(bool)
	closed, dimmed, badge bool
	tray                  *native.TrayIcon
	shortcut              *native.ShortcutRegistration
	assertion             *native.PowerAssertion
}

func (s *systemState) alive() bool { return !s.closed && !s.window.Closed }
func (s *systemState) showWindow() {
	if s.alive() {
		s.window.Show()
		s.window.Focus()
	}
}

func (s *systemState) run(action systemAction) {
	if !s.alive() || s.busy() {
		return
	}
	s.setBusy(true)
	s.status(action.label + "…")
	action.operation(func(message string, err error) {
		if !s.alive() {
			return
		}
		s.setBusy(false)
		if err != nil {
			s.status(action.label + " failed: " + err.Error())
		} else {
			s.status(message)
		}
	})
}

func (s *systemState) dispose() {
	s.closed = true
	if s.assertion != nil {
		_, err := s.assertion.Release()
		logError(err)
	}
	if s.shortcut != nil {
		s.shortcut.Unregister(logError)
	}
	if s.tray != nil {
		s.tray.Destroy(logError)
	}
	if s.badge {
		native.Desktop.SetDockBadge("")
	}
}

func logError(err error) {
	if err != nil {
		log.Print(err)
	}
}

func report[T any](done completion) func(T, error) {
	return func(value T, err error) {
		if err != nil {
			done("", err)
			return
		}
		data, err := json.MarshalIndent(value, "", "  ")
		done(string(data), err)
	}
}

func after[T any](s *systemState, done completion, next func(T)) func(T, error) {
	return func(value T, err error) {
		if !s.alive() {
			return
		}
		if err != nil {
			done("", err)
			return
		}
		next(value)
	}
}

func finished(done completion, message string) func(error) {
	return func(err error) { done(message, err) }
}

var buttonStyle = ui.Styles(
	ui.Display("flex"),
	ui.Height(38),
	ui.AlignItems("center"),
	ui.JustifyContent("center"),
	ui.PaddingLeft(14),
	ui.PaddingRight(14),
	ui.BackgroundColor("#252b37"),
	ui.Color("#f4f7fb"),
	ui.BorderColor("#394253"),
	ui.BorderWidth(1),
	ui.BorderRadius(8),
	ui.Cursor("default"),
	ui.AppRegion("no-drag"),
	ui.UserSelect("none"),
	ui.Hover(ui.BackgroundColor("#30394a")),
)
