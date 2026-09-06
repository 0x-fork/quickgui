import type { NativeNode } from "@quickgui/native";
import { Button, For, Show, Text, View, createSignal } from "@quickgui/ui";
import { Avatar } from "@quickgui/ui/base-ui";
import { Checkbox, CheckboxGroup, type CheckedState } from "@quickgui/ui/controls";
import { AlertDialog, Dialog } from "@quickgui/ui/dialog";
import { Accordion, Collapsible } from "@quickgui/ui/disclosure";
import { Input } from "@quickgui/ui";
import type { Style } from "@quickgui/ui";

import { controlStyle, inputStyle, overlayFill, p, popupStyle } from "../theme.ts";
import { Btn, Label, Muted, Note, Panel, Row } from "../ui.tsx";

/* -------------------------------------------------------------------------------------------- *
 * Accordion
 * -------------------------------------------------------------------------------------------- */

interface AccordionEntry {
  value: string;
  title: string;
  body: string;
}

const accordionEntries: AccordionEntry[] = [
  {
    value: "declared",
    title: "Everything is declared",
    body: "The application declares values and elements; the Rust core owns roles, focus, and deadlines.",
  },
  {
    value: "panels",
    title: "Closed panels are not mounted",
    body: "A closed Accordion.Panel contributes no layout, paint, input, or accessibility node.",
  },
  {
    value: "heading",
    title: "Heading level is the core's",
    body: "headingLevel is clamped by the core to 1 through 6 and projected on every header.",
  },
];

export function AccordionDemo(): NativeNode {
  const [open, setOpen] = createSignal<string[]>(["declared"]);
  return (
    <Panel title="Accordion" hint="multiple + headingLevel. Click a header; the open value set below is the core's answer.">
      <Accordion.Root
        multiple
        headingLevel={3}
        value={open}
        onValueChange={(next) => setOpen(next)}
        style={{ display: "flex", flexDirection: "column", gap: 6 }}
      >
        <For each={() => accordionEntries}>
          {(entry, index) => (
            <Accordion.Item
              value={entry.value}
              index={index()}
              style={{
                display: "flex",
                flexDirection: "column",
                borderRadius: 10,
                borderWidth: 1,
                borderColor: p().border,
                backgroundColor: p().panelAlt,
                overflow: "hidden",
              }}
            >
              <Accordion.Header style={{ display: "flex", width: "100%" }}>
                <Accordion.Trigger
                  style={{
                    display: "flex",
                    flexDirection: "row",
                    alignItems: "center",
                    justifyContent: "space-between",
                    width: "100%",
                    height: 34,
                    paddingLeft: 12,
                    paddingRight: 12,
                    // The ring is drawn inside the header with the item's own corner radius, so
                    // it never lands on top of the rounded 1px border around the item.
                    borderRadius: 9,
                    hover: { backgroundColor: p().controlHover },
                    focus: { outline: "2px solid " + p().accent },
                    outlineOffset: -2,
                  }}
                >
                  <Text style={{ fontSize: 12, color: p().ink }}>{entry.title}</Text>
                  <Text style={{ fontSize: 12, color: p().muted }}>{open().includes(entry.value) ? "−" : "+"}</Text>
                </Accordion.Trigger>
              </Accordion.Header>
              <Accordion.Panel style={{ paddingLeft: 12, paddingRight: 12, paddingBottom: 10 }}>
                <Text style={{ fontSize: 12, color: p().muted, lineHeight: 17 }}>{entry.body}</Text>
              </Accordion.Panel>
            </Accordion.Item>
          )}
        </For>
      </Accordion.Root>
      <Note text={"open: " + (open().length === 0 ? "none" : open().join(", "))} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Alert dialog
 * -------------------------------------------------------------------------------------------- */

export function AlertDialogDemo(): NativeNode {
  const [open, setOpen] = createSignal(false);
  const [outcome, setOutcome] = createSignal("nothing yet");
  const [reason, setReason] = createSignal("—");
  return (
    <Panel
      title="Alert dialog"
      hint="An alert dialog refuses a backdrop dismissal by default, so a consequential choice needs a real answer. The portal covers the whole window."
    >
      <AlertDialog.Root
        open={open}
        onOpenChange={(next, details) => {
          setOpen(next);
          setReason(details.reason);
          if (!next && outcome() === "nothing yet") setOutcome("dismissed");
        }}
      >
        <AlertDialog.Trigger style={controlStyle()}>
          <Text style={{ fontSize: 12, color: p().danger }}>Delete branch…</Text>
        </AlertDialog.Trigger>
        <AlertDialog.Portal style={[overlayFill(), { display: "flex", alignItems: "center", justifyContent: "center" }]}>
          <AlertDialog.Backdrop style={[overlayFill(), { backgroundColor: p().backdrop }]} />
          <AlertDialog.Popup style={[popupStyle(), { width: 340 }]}>
            <AlertDialog.Title>
              <Text style={{ fontSize: 15, fontWeight: 700, color: p().ink }}>Delete “electron-parity”?</Text>
            </AlertDialog.Title>
            <AlertDialog.Description>
              <Text style={{ fontSize: 12, color: p().muted, lineHeight: 17 }}>
                The core traps focus here and restores it to the trigger on every dismissal path.
              </Text>
            </AlertDialog.Description>
            <Row>
              <AlertDialog.Close style={controlStyle()}>
                <Text style={{ fontSize: 12, color: p().ink }}>Cancel</Text>
              </AlertDialog.Close>
              <Button
                style={[
                  controlStyle(),
                  { backgroundColor: p().danger, borderColor: p().danger, hover: { backgroundColor: p().dangerHover } },
                ]}
                onClick={() => {
                  setOutcome("deleted");
                  setOpen(false);
                }}
              >
                <Text style={{ fontSize: 12, color: "#ffffff" }}>Delete</Text>
              </Button>
            </Row>
          </AlertDialog.Popup>
        </AlertDialog.Portal>
      </AlertDialog.Root>
      <Note text={"open " + String(open()) + " · last reason " + reason() + " · outcome " + outcome()} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Avatar
 * -------------------------------------------------------------------------------------------- */

function avatarShell(): Style {
  return {
    width: 44,
    height: 44,
    borderRadius: 22,
    display: "flex",
    alignItems: "center",
    justifyContent: "center",
    backgroundColor: p().selection,
    borderColor: p().border,
    borderWidth: 1,
    overflow: "hidden",
  };
}

export function AvatarDemo(): NativeNode {
  const [missing, setMissing] = createSignal("idle");
  const [initials, setInitials] = createSignal("idle");
  return (
    <Panel
      title="Avatar"
      hint="The core mounts exactly one of the image and the fallback, so the name is announced once. A load failure is reported, never guessed."
    >
      <Row>
        <Avatar.Root ariaLabel="Ada Lovelace" onLoadingStatusChange={(status) => setMissing(status)} style={avatarShell()}>
          <Avatar.Image src="./avatar-does-not-exist.png" style={{ width: 44, height: 44 }} />
          <Avatar.Fallback delay={120}>
            <Text style={{ fontSize: 14, fontWeight: 700, color: p().ink }}>AL</Text>
          </Avatar.Fallback>
        </Avatar.Root>
        <Avatar.Root ariaLabel="Grace Hopper" onLoadingStatusChange={(status) => setInitials(status)} style={avatarShell()}>
          <Avatar.Fallback>
            <Text style={{ fontSize: 14, fontWeight: 700, color: p().ink }}>GH</Text>
          </Avatar.Fallback>
        </Avatar.Root>
      </Row>
      <Note text={"with image: " + missing() + " · fallback only: " + initials()} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Button
 * -------------------------------------------------------------------------------------------- */

export function ButtonDemo(): NativeNode {
  const [clicks, setClicks] = createSignal(0);
  const [last, setLast] = createSignal("nothing yet");
  return (
    <Panel
      title="Button"
      hint="An ordinary native button node: the core owns the press, the focus ring, the cursor, and the window-drag exclusion."
    >
      <Row>
        <Btn
          label="Primary"
          primary
          onClick={() => {
            setClicks(clicks() + 1);
            setLast("primary");
          }}
        />
        <Btn
          label="Secondary"
          onClick={() => {
            setClicks(clicks() + 1);
            setLast("secondary");
          }}
        />
        <Btn label="Disabled" disabled onClick={() => setLast("never")} />
        <Button style={controlStyle()} onDoubleClick={() => setLast("double click")} onClick={() => setLast("single click")}>
          <Text style={{ fontSize: 12, color: p().ink }}>Double-click me</Text>
        </Button>
      </Row>
      <Note text={"clicks " + String(clicks()) + " · last " + last()} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Checkbox
 * -------------------------------------------------------------------------------------------- */

function checkboxRow(): Style {
  return {
    display: "flex",
    flexDirection: "row",
    alignItems: "center",
    gap: 8,
    height: 28,
    paddingLeft: 8,
    paddingRight: 10,
    borderRadius: 8,
    hover: { backgroundColor: p().controlHover },
    focus: { outline: "2px solid " + p().accent },
  };
}

function tick(checked: CheckedState): Style {
  return {
    width: 16,
    height: 16,
    borderRadius: 5,
    alignItems: "center",
    justifyContent: "center",
    borderWidth: 1,
    borderColor: checked === false ? p().border : p().accent,
    backgroundColor: checked === false ? p().control : p().accent,
  };
}

function foldChildren(kids: boolean[]): CheckedState {
  let on = 0;
  for (const kid of kids) if (kid) on += 1;
  return on === kids.length ? true : on > 0 ? "indeterminate" : false;
}

export function CheckboxDemo(): NativeNode {
  const [notify, setNotify] = createSignal<CheckedState>("indeterminate");
  const [locked] = createSignal<CheckedState>(true);
  const [kids, setKids] = createSignal<boolean[]>([true, false]);
  const kidNames = ["Analytics", "Crash reports"];
  return (
    <Panel
      title="Checkbox"
      hint="Tri-state on/off/mixed, a read-only box that keeps its Tab stop, and a standalone parent whose mixed state the core derives from childrenChecked."
    >
      <Checkbox.Root checked={notify} onCheckedChange={(next) => setNotify(next)} style={checkboxRow()}>
        <Checkbox.Indicator style={tick(notify())}>
          <Text style={{ fontSize: 11, color: p().onAccent }}>{notify() === true ? "✓" : notify() === "indeterminate" ? "–" : ""}</Text>
        </Checkbox.Indicator>
        <Label text="Email me about releases" />
      </Checkbox.Root>

      <Checkbox.Root checked={locked} readOnly style={checkboxRow()}>
        <Checkbox.Indicator style={tick(locked())}>
          <Text style={{ fontSize: 11, color: p().onAccent }}>✓</Text>
        </Checkbox.Indicator>
        <Muted text="Read-only: focusable, refuses changes" />
      </Checkbox.Root>

      <View style={{ height: 1, backgroundColor: p().border }} />

      <Checkbox.Root parent childrenChecked={kids} style={checkboxRow()}>
        <Checkbox.Indicator style={tick(foldChildren(kids()))} />
        <Label text="Parent, derived from its children" />
      </Checkbox.Root>
      <Row>
        <For each={() => kidNames}>
          {(name, index) => (
            <Checkbox.Root
              checked={(): CheckedState => kids()[index()] === true}
              onCheckedChange={(next) => {
                const updated: boolean[] = [];
                const current = kids();
                for (let i = 0; i < current.length; i += 1) updated.push(i === index() ? next : current[i]!);
                setKids(updated);
              }}
              style={checkboxRow()}
            >
              <Checkbox.Indicator style={tick(kids()[index()] === true)} />
              <Label text={name} />
            </Checkbox.Root>
          )}
        </For>
      </Row>
      <Note text={"notify " + String(notify()) + " · locked " + String(locked()) + " · children [" + kids().join(", ") + "]"} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Checkbox group
 * -------------------------------------------------------------------------------------------- */

const allColors = ["red", "green", "blue", "violet"];

export function CheckboxGroupDemo(): NativeNode {
  const [colors, setColors] = createSignal<string[]>(["green"]);
  return (
    <Panel
      title="Checkbox group"
      hint="The parent has no retained value of its own; the core folds the children into on, mixed, or off, and keeps checked values in the declared order."
    >
      <CheckboxGroup.Root
        allValues={allColors}
        value={colors}
        onValueChange={(next) => setColors(next)}
        style={{ display: "flex", flexDirection: "column", gap: 4 }}
      >
        <Checkbox.Root parent style={checkboxRow()}>
          <Checkbox.Indicator
            style={{
              width: 16,
              height: 16,
              borderRadius: 5,
              borderWidth: 1,
              borderColor: p().accent,
              backgroundColor: colors().length > 0 ? p().accent : p().control,
            }}
          />
          <Label text="All colours" />
        </Checkbox.Root>
        <For each={() => allColors}>
          {(value) => (
            <Checkbox.Root value={value} style={[checkboxRow(), { marginLeft: 18 }]}>
              <Checkbox.Indicator
                style={{
                  width: 16,
                  height: 16,
                  borderRadius: 5,
                  borderWidth: 1,
                  borderColor: colors().includes(value) ? p().accent : p().border,
                  backgroundColor: colors().includes(value) ? p().accent : p().control,
                }}
              />
              <Label text={value} />
            </Checkbox.Root>
          )}
        </For>
      </CheckboxGroup.Root>
      <Note text={"checked: " + (colors().length === 0 ? "none" : colors().join(", "))} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Collapsible
 * -------------------------------------------------------------------------------------------- */

function collapsibleBox(): Style {
  return {
    display: "flex",
    flexDirection: "column",
    gap: 6,
    borderRadius: 10,
    borderWidth: 1,
    borderColor: p().border,
    padding: 10,
    backgroundColor: p().panelAlt,
  };
}

export function CollapsibleDemo(): NativeNode {
  const [open, setOpen] = createSignal(false);
  const [kept, setKept] = createSignal(true);
  return (
    <Panel
      title="Collapsible"
      hint="One disclosure. Its panel is omitted entirely while closed unless keepMounted retains it as display:none."
    >
      <Collapsible.Root open={open} onOpenChange={(next) => setOpen(next)} style={collapsibleBox()}>
        <Collapsible.Trigger style={controlStyle()}>
          <Text style={{ fontSize: 12, color: p().ink }}>{open() ? "Hide advanced options" : "Show advanced options"}</Text>
        </Collapsible.Trigger>
        <Collapsible.Panel style={{ paddingTop: 4 }}>
          <Text style={{ fontSize: 12, color: p().muted, lineHeight: 17 }}>
            While closed this panel contributes no layout, paint, input, or accessibility node at all.
          </Text>
        </Collapsible.Panel>
      </Collapsible.Root>

      <Collapsible.Root open={kept} onOpenChange={(next) => setKept(next)} keepMounted style={collapsibleBox()}>
        <Collapsible.Trigger style={controlStyle()}>
          <Text style={{ fontSize: 12, color: p().ink }}>{"keepMounted · " + (kept() ? "open" : "closed")}</Text>
        </Collapsible.Trigger>
        <Collapsible.Panel style={{ paddingTop: 4 }}>
          <Muted text="Retained as display:none instead of omitted." />
        </Collapsible.Panel>
      </Collapsible.Root>

      <Note text={"plain " + String(open()) + " · keepMounted " + String(kept())} />
    </Panel>
  );
}

/* -------------------------------------------------------------------------------------------- *
 * Dialog
 * -------------------------------------------------------------------------------------------- */

export function DialogDemo(): NativeNode {
  const [open, setOpen] = createSignal(false);
  const [reason, setReason] = createSignal("—");
  const [completed, setCompleted] = createSignal("—");
  const [notes, setNotes] = createSignal("");
  return (
    <Panel
      title="Dialog"
      hint="An in-window dialog with the core's own focus trap. exitDuration is the exact deadline the surface stays mounted for, and Dialog.Viewport is the scrollable body."
    >
      <Dialog.Root
        open={open}
        exitDuration={120}
        onOpenChange={(next, details) => {
          setOpen(next);
          setReason(details.reason);
        }}
        onOpenChangeComplete={(next) => setCompleted(next ? "opened" : "closed")}
      >
        <Dialog.Trigger style={controlStyle()}>
          <Text style={{ fontSize: 12, color: p().ink }}>Open dialog</Text>
        </Dialog.Trigger>
        <Dialog.Portal style={[overlayFill(), { display: "flex", alignItems: "center", justifyContent: "center" }]}>
          <Dialog.Backdrop style={[overlayFill(), { backgroundColor: p().backdrop }]} />
          <Dialog.Popup style={[popupStyle(), { width: 360 }]}>
            <Dialog.Title>
              <Text style={{ fontSize: 15, fontWeight: 700, color: p().ink }}>Publish this build?</Text>
            </Dialog.Title>
            <Dialog.Description>
              <Text style={{ fontSize: 12, color: p().muted, lineHeight: 17 }}>
                Escape, the backdrop, and the close control all restore focus to the trigger.
              </Text>
            </Dialog.Description>
            {/* Dialog.Viewport is the scrollable dialog body: the core owns its overflow. */}
            <Dialog.Viewport style={{ display: "flex", flexDirection: "column", gap: 8, maxHeight: 160, padding: 4 }}>
              <Input
                value={notes()}
                multiline
                placeholder="Release notes"
                onInput={(event) => setNotes(event.value ?? "")}
                style={[inputStyle(), { width: "100%", height: 64, paddingTop: 6 }]}
              />
            </Dialog.Viewport>
            <Row>
              <Dialog.Close style={controlStyle()}>
                <Text style={{ fontSize: 12, color: p().ink }}>Cancel</Text>
              </Dialog.Close>
              <Btn label="Publish" primary onClick={() => setOpen(false)} />
            </Row>
          </Dialog.Popup>
        </Dialog.Portal>
      </Dialog.Root>
      <Note
        text={"open " + String(open()) + " · reason " + reason() + " · transition " + completed() + " · notes " + String(notes().length) + " chars"}
      />
    </Panel>
  );
}
