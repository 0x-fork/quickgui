package native

import (
	"slices"

	"github.com/egoist/quickgui/go/host"
	"github.com/egoist/quickgui/go/protocol"
	"github.com/egoist/quickgui/go/reactive"
)

type EventListener func(*Event)

type Listener struct {
	Type     int
	Listener EventListener
}

// Event is one native event delivered to a mounted node.
type Event struct {
	Type               int
	Target             *Node
	CurrentTarget      *Node
	DefaultPrevented   bool
	PropagationStopped bool
	Value              string
	hasValue           bool
}

func (e *Event) PreventDefault() { e.DefaultPrevented = true }

func (e *Event) StopPropagation() { e.PropagationStopped = true }

func (e *Event) ValueOK() (string, bool) { return e.Value, e.hasValue }

var nextNodeID uint32 = 1

func allocateNodeID() uint32 {
	if nextNodeID >= 0xffff_ffff {
		panic("QuickGUI native node id space exhausted")
	}
	id := nextNodeID
	nextNodeID++
	return id
}

var pendingFlush []*NodeHost

// NodeHost is one window's mutation sink.
type NodeHost struct {
	AppID          uint32
	NativeID       uint32
	Nodes          map[uint32]*Node
	Batch          *protocol.Batch
	FlushScheduled bool
	Closed         bool
	NativeReady    bool
}

func NewNodeHost(appID, nativeID uint32) *NodeHost {
	return &NodeHost{
		AppID:    appID,
		NativeID: nativeID,
		Nodes:    map[uint32]*Node{},
		Batch:    protocol.NewBatch(),
	}
}

func (h *NodeHost) ScheduleFlush() {
	if h.FlushScheduled || h.Closed || !h.NativeReady {
		return
	}
	h.FlushScheduled = true
	pendingFlush = append(pendingFlush, h)
}

func (h *NodeHost) Flush() {
	if h.Closed || !h.NativeReady {
		return
	}
	h.FlushScheduled = false
	if h.Batch.Empty() {
		return
	}
	bytes := h.Batch.Finish()
	h.Batch = protocol.NewBatch()
	host.Current.ApplyBatch(h.AppID, h.NativeID, bytes)
}

func (h *NodeHost) TakeBatch() []byte {
	h.FlushScheduled = false
	batch := h.Batch
	h.Batch = protocol.NewBatch()
	return batch.Finish()
}

func (h *NodeHost) FocusNode(node *Node) bool {
	if h.Closed || node.Host != h {
		return false
	}
	h.Flush()
	host.Current.FocusNode(h.AppID, h.NativeID, node.ID)
	return true
}

// FlushPending commits every window that mutated during the current application turn.
func FlushPending() {
	hosts := pendingFlush
	pendingFlush = nil
	for _, host := range hosts {
		host.FlushScheduled = false
		host.Flush()
	}
}

// Node is a thin application-side retained node.
type Node struct {
	ID              uint32
	Tag             uint8
	Text            string
	Parent          *Node
	Children        []*Node
	Host            *NodeHost
	Pending         *protocol.Batch
	Listeners       []*Listener
	Group           []*Node
	Removed         bool
	properties      map[uint16]any
	bindings        *reactive.Owner
	propertyCapture map[uint16]struct{}
	bindingScope    *reactive.Owner
}

// BindingOwner holds bindings and conditional content until this node is removed.
func (n *Node) BindingOwner() *reactive.Owner {
	if n.Removed {
		panic("a removed QuickGUI node cannot own bindings")
	}
	if n.bindings == nil {
		n.bindings = reactive.NewOwner(reactive.GetOwner())
	}
	return n.bindings
}

// Bind creates a reactive binding that is disposed with this node.
func (n *Node) Bind(fn func()) {
	owner := n.bindingScope
	if owner == nil {
		owner = n.BindingOwner()
	}
	reactive.RunWithOwner(owner, func() struct{} {
		reactive.CreateRenderEffect(fn)
		return struct{}{}
	})
}

// BindProperties replaces one reactive property declaration. Properties omitted
// on the next run are cleared; nested bindings are disposed before reevaluation.
// Children are mounted separately and are never rebuilt by this binding.
func (n *Node) BindProperties(declare func()) {
	var previous map[uint16]struct{}
	n.Bind(func() {
		next := make(map[uint16]struct{})
		func() {
			capture, scope := n.propertyCapture, n.bindingScope
			n.propertyCapture = next
			n.bindingScope = reactive.GetOwner()
			defer func() { n.propertyCapture, n.bindingScope = capture, scope }()
			declare()
		}()
		for property := range previous {
			if _, retained := next[property]; !retained {
				ClearProperty(n, property)
			}
		}
		previous = next
	})
}

func (n *Node) Focus() bool {
	if n.Host == nil {
		return false
	}
	return n.Host.FocusNode(n)
}

func batchFor(node *Node) *protocol.Batch {
	if node.Removed {
		panic("a removed QuickGUI node cannot be mutated")
	}
	current := node
	for current != nil {
		if current.Host != nil {
			current.Host.ScheduleFlush()
			return current.Host.Batch
		}
		if current.Pending != nil {
			return current.Pending
		}
		current = current.Parent
	}
	panic("a removed QuickGUI node cannot be mutated")
}

func CreateElement(tag uint8) *Node {
	node := &Node{ID: allocateNodeID(), Tag: tag}
	pending := protocol.NewBatch()
	pending.CreateElement(node.ID, tag)
	node.Pending = pending
	collectCreatedNode(node)
	return node
}

func CreateText(value string) *Node {
	node := &Node{ID: allocateNodeID(), Tag: protocol.TagText, Text: value}
	pending := protocol.NewBatch()
	pending.CreateText(node.ID, value)
	node.Pending = pending
	collectCreatedNode(node)
	return node
}

func CreateSentinel() *Node {
	node := &Node{ID: allocateNodeID(), Tag: protocol.TagSentinel}
	pending := protocol.NewBatch()
	pending.CreateSentinel(node.ID)
	node.Pending = pending
	collectCreatedNode(node)
	return node
}

func CreateRootNode(host *NodeHost, id uint32) *Node {
	node := &Node{ID: id, Tag: protocol.TagView, Host: host}
	host.Nodes[id] = node
	return node
}

func ReplaceText(node *Node, value string) {
	if node.Tag != protocol.TagText {
		panic("replaceText expects a text node")
	}
	if node.Text == value {
		return
	}
	node.Text = value
	batchFor(node).ReplaceText(node.ID, value)
}

func recordProperty(node *Node, property uint16, value any) bool {
	if node.Removed {
		panic("a removed QuickGUI node cannot be mutated")
	}
	if node.propertyCapture != nil {
		node.propertyCapture[property] = struct{}{}
	}
	if previous, exists := node.properties[property]; exists && previous == value {
		return false
	}
	if node.properties == nil {
		node.properties = make(map[uint16]any)
	}
	node.properties[property] = value
	return true
}

func SetString(node *Node, property uint16, value string) {
	if !recordProperty(node, property, value) {
		return
	}
	batchFor(node).SetString(node.ID, property, value)
}

func SetNumber(node *Node, property uint16, value float32) {
	if !recordProperty(node, property, value) {
		return
	}
	batchFor(node).SetNumber(node.ID, property, value)
}

func SetBoolean(node *Node, property uint16, value bool) {
	if !recordProperty(node, property, value) {
		return
	}
	batchFor(node).SetBoolean(node.ID, property, value)
}

func SetColor(node *Node, property uint16, value uint32) {
	if !recordProperty(node, property, value) {
		return
	}
	batchFor(node).SetColor(node.ID, property, value)
}

func ClearProperty(node *Node, property uint16) {
	if node.Removed {
		panic("a removed QuickGUI node cannot be mutated")
	}
	if node.propertyCapture != nil {
		node.propertyCapture[property] = struct{}{}
	}
	if _, exists := node.properties[property]; !exists {
		return
	}
	delete(node.properties, property)
	batchFor(node).ClearProperty(node.ID, property)
}

func hasListenerSharing(node *Node, eventType int) bool {
	for _, entry := range node.Listeners {
		if protocol.SharesListenerProperty(entry.Type, eventType) {
			return true
		}
	}
	return false
}

func SetEventListener(node *Node, eventType int, listener EventListener) {
	index := -1
	for i, entry := range node.Listeners {
		if entry.Type == eventType {
			index = i
			break
		}
	}
	if listener == nil {
		if index >= 0 {
			node.Listeners = slices.Delete(node.Listeners, index, index+1)
		}
	} else if index >= 0 {
		node.Listeners[index] = &Listener{Type: eventType, Listener: listener}
	} else {
		node.Listeners = append(node.Listeners, &Listener{Type: eventType, Listener: listener})
	}
	property := protocol.ListenerPropertyFor(eventType)
	if property == 0 {
		return
	}
	SetBoolean(node, property, hasListenerSharing(node, eventType))
}

func materialize(node *Node, host *NodeHost) {
	node.Host = host
	host.Nodes[node.ID] = node
	for _, child := range node.Children {
		materialize(child, host)
	}
}

func dematerialize(node *Node) {
	if node.Host != nil {
		delete(node.Host.Nodes, node.ID)
	}
	node.Host = nil
	for _, child := range node.Children {
		dematerialize(child)
	}
}

func retire(node *Node) {
	if node.Removed {
		return
	}
	reactive.DisposeOwner(node.bindings)
	node.bindings = nil
	node.properties = nil
	node.Removed = true
	node.Pending = nil
	node.Listeners = nil
	for _, child := range node.Children {
		child.Parent = nil
		retire(child)
	}
	node.Children = nil
	node.Group = nil
}

func indexOfChild(parent, node *Node) int {
	for i, child := range parent.Children {
		if child == node {
			return i
		}
	}
	return -1
}

func insertAt(array []*Node, index int, item *Node) []*Node {
	array = append(array, nil)
	copy(array[index+1:], array[index:])
	array[index] = item
	return array
}

// InsertNode inserts node into parent before anchor, or at the end.
func InsertNode(parent, node, anchor *Node) {
	if node.Removed {
		panic("a removed QuickGUI node cannot be inserted again")
	}
	if parent.Removed {
		panic("a removed QuickGUI node cannot contain children")
	}
	for ancestor := parent; ancestor != nil; ancestor = ancestor.Parent {
		if ancestor == node {
			panic("a native node cannot contain itself or an ancestor")
		}
	}
	if node.Host != nil && node.Host != parent.Host {
		panic("a native node cannot move between QuickGUI windows")
	}
	if anchor != nil && anchor.Parent != parent {
		panic("anchor is not a child of parent")
	}
	if anchor == node && node.Parent == parent {
		return
	}
	if node.Group != nil {
		for _, member := range node.Group {
			InsertNode(parent, member, anchor)
		}
	}
	if node.Parent != nil {
		if previous := indexOfChild(node.Parent, node); previous >= 0 {
			node.Parent.Children = slices.Delete(node.Parent.Children, previous, previous+1)
		}
	}
	index := len(parent.Children)
	if anchor != nil {
		index = indexOfChild(parent, anchor)
		if index < 0 {
			index = len(parent.Children)
		}
	}
	parent.Children = insertAt(parent.Children, index, node)
	node.Parent = parent
	batch := batchFor(parent)
	if node.Pending != nil {
		batch.Append(node.Pending)
		node.Pending = nil
	}
	before := protocol.NoAnchor
	if anchor != nil {
		before = anchor.ID
	}
	batch.Insert(parent.ID, node.ID, before)
	if parent.Host != nil && node.Host != parent.Host {
		if node.Host != nil {
			panic("a native node cannot move between QuickGUI windows")
		}
		materialize(node, parent.Host)
	}
}

func RemoveNode(parent, node *Node) {
	if node.Parent != parent {
		return
	}
	if node.Group != nil {
		for _, member := range node.Group {
			RemoveNode(parent, member)
		}
	}
	if index := indexOfChild(parent, node); index >= 0 {
		parent.Children = slices.Delete(parent.Children, index, index+1)
	}
	node.Parent = nil
	batchFor(parent).Remove(parent.ID, node.ID)
	dematerialize(node)
	retire(node)
}

func DispatchEvent(host *NodeHost, eventType int, targetID uint32, value string, hasValue bool) {
	target, ok := host.Nodes[targetID]
	if !ok {
		return
	}
	event := &Event{Type: eventType, Target: target, CurrentTarget: target, Value: value, hasValue: hasValue}
	if eventType == protocol.EventMouseEnter || eventType == protocol.EventMouseLeave {
		invokeListeners(target, event)
		return
	}
	current := target
	for current != nil {
		event.CurrentTarget = current
		invokeListeners(current, event)
		if event.PropagationStopped {
			break
		}
		current = current.Parent
	}
}

func invokeListeners(node *Node, event *Event) {
	for _, entry := range node.Listeners {
		if entry.Type == event.Type {
			entry.Listener(event)
			return
		}
	}
}

func ResetTreeStateForTests() {
	nextNodeID = 1
	pendingFlush = nil
}
