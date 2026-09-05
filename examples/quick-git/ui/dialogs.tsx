import { Dialog as NativeDialog } from "@quickgui/native";
import { Checkbox, Dialog, Input, Select, Text, View, keyEventFromEvent } from "@quickgui/solid";
import { Show, createMemo, createSignal } from "solid-js";

import { branchNameProblem } from "../git/refs.ts";
import { useApp } from "./context.tsx";
import { Icon } from "./icons.tsx";
import { PushButton } from "./primitives.tsx";

export function Dialogs() {
  const app = useApp();
  return (
    <>
      <Show when={app.dialog()?.kind === "new-branch"}>
        <NewBranchDialog />
      </Show>
      <Show when={app.dialog()?.kind === "new-worktree"}>
        <NewWorktreeDialog />
      </Show>
      <Show when={app.dialog()?.kind === "stash"}>
        <StashDialog />
      </Show>
    </>
  );
}

function DialogFrame(props: {
  title: string;
  description?: string;
  children: unknown;
  actions: unknown;
}) {
  const app = useApp();
  return (
    <Dialog.Root open onOpenChange={(open) => !open && app.closeDialog()} exitDuration={0}>
      <Dialog.Portal style={app.styles().dialogPortal}>
        <Dialog.Backdrop style={app.styles().dialogBackdrop} />
        <Dialog.Popup
          style={app.styles().dialogPopup}
          onKeyDown={(event) => {
            if (keyEventFromEvent(event)?.key === "Escape") app.closeDialog();
          }}
        >
          <Dialog.Title style={app.styles().dialogTitle}>{props.title}</Dialog.Title>
          <Show when={props.description}>
            <Dialog.Description style={app.styles().dialogDescription}>{props.description}</Dialog.Description>
          </Show>
          <Dialog.Viewport style={{ display: "flex", flexDirection: "column", gap: 12, padding: 2 }}>{props.children as never}</Dialog.Viewport>
          <View style={{ display: "flex", flexDirection: "row", justifyContent: "flex-end", gap: 8, marginTop: 4 }}>
            <Dialog.Close style={app.styles().button("secondary")}>
              <Text>Cancel</Text>
            </Dialog.Close>
            {props.actions as never}
          </View>
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

function Field(props: { label: string; children: unknown; hint?: string | undefined; error?: string | undefined }) {
  const app = useApp();
  return (
    <View style={{ display: "flex", flexDirection: "column", gap: 5 }}>
      <Text style={app.styles().fieldLabel}>{props.label}</Text>
      {props.children as never}
      <Show when={props.error}>
        <Text style={{ fontSize: 11.5, color: app.theme().danger }}>{props.error}</Text>
      </Show>
      <Show when={!props.error && props.hint}>
        <Text style={{ fontSize: 11.5, color: app.theme().textTertiary }}>{props.hint}</Text>
      </Show>
    </View>
  );
}

function NewBranchDialog() {
  const app = useApp();
  const store = app.store;
  const request = app.dialog();
  const initialBase = (request?.kind === "new-branch" && request.from) || store.status()?.branch || "HEAD";
  const [name, setName] = createSignal("");
  const [base, setBase] = createSignal(initialBase);
  const [checkout, setCheckout] = createSignal(true);
  const problem = createMemo(() => (name() ? branchNameProblem(name().trim()) : undefined));
  const exists = createMemo(() => store.refs().local.some((branch) => branch.name === name().trim()));
  const baseOptions = createMemo(() => {
    const names = new Set<string>(["HEAD"]);
    for (const branch of store.refs().local) names.add(branch.name);
    for (const branch of store.refs().remote) names.add(branch.name);
    names.add(initialBase);
    return [...names].map((value) => ({ value, label: value }));
  });
  const valid = () => name().trim().length > 0 && !problem() && !exists();

  async function submit(): Promise<void> {
    if (!valid()) return;
    app.closeDialog();
    await store.createBranch(name().trim(), { from: base(), checkout: checkout() });
  }

  return (
    <DialogFrame
      title="New Branch"
      actions={<PushButton ui={app} kind="primary" label={checkout() ? "Create and Switch" : "Create"} disabled={!valid()} onClick={() => void submit()} />}
    >
      <Field label="Name" error={problem() ?? (exists() ? "A branch with this name already exists." : undefined)}>
        <Input
          autoFocus
          value={name()}
          placeholder="feature/great-idea"
          onInput={(event) => setName(event.value ?? "")}
          onSubmit={() => void submit()}
          style={app.styles().input}
        />
      </Field>
      <Field label="Based on">
        <Select.Root
          scope="new-branch-base"
          ariaLabel="Base branch"
          value={base()}
          items={baseOptions()}
          onValueChange={(value) => value && setBase(value)}
          appearance={pickerAppearance(app)}
          style={[app.styles().input, { flexDirection: "row", alignItems: "center", justifyContent: "space-between", gap: 8 }]}
        >
          <Text style={{ fontSize: 13, color: app.theme().text }}>{base()}</Text>
          <Icon name="chevron-down" size={12} color={app.theme().textTertiary} />
        </Select.Root>
      </Field>
      <CheckRow label="Switch to the new branch" checked={checkout()} onChange={setCheckout} />
    </DialogFrame>
  );
}

function NewWorktreeDialog() {
  const app = useApp();
  const store = app.store;
  const request = app.dialog();
  const presetBranch = request?.kind === "new-worktree" ? request.branch : undefined;
  const [mode, setMode] = createSignal<"new" | "existing">(presetBranch ? "existing" : "new");
  const [name, setName] = createSignal("");
  const [existing, setExisting] = createSignal(presetBranch ?? "");
  const [base, setBase] = createSignal(store.status()?.branch ?? "HEAD");
  const [path, setPath] = createSignal(presetBranch ? store.suggestWorktreePath(presetBranch) : "");
  const [pathTouched, setPathTouched] = createSignal(false);
  const [launch, setLaunch] = createSignal<"none" | "codex" | "claude">("none");

  const availableBranches = createMemo(() =>
    store.refs().local.filter((branch) => !branch.worktreePath).map((branch) => ({ value: branch.name, label: branch.name })),
  );
  const baseOptions = createMemo(() => {
    const names = new Set<string>(["HEAD"]);
    for (const branch of store.refs().local) names.add(branch.name);
    for (const branch of store.refs().remote) names.add(branch.name);
    return [...names].map((value) => ({ value, label: value }));
  });
  const branchForPath = () => (mode() === "new" ? name().trim() : existing());
  const effectivePath = () => (pathTouched() && path().trim() ? path().trim() : branchForPath() ? store.suggestWorktreePath(branchForPath()) : "");
  const problem = createMemo(() => (mode() === "new" && name() ? branchNameProblem(name().trim()) : undefined));
  const duplicate = createMemo(() => mode() === "new" && store.refs().local.some((branch) => branch.name === name().trim()));
  const valid = () => branchForPath().length > 0 && !problem() && !duplicate() && effectivePath().length > 0;
  const agents = () => store.agents();

  async function submit(): Promise<void> {
    if (!valid()) return;
    app.closeDialog();
    const target = effectivePath();
    const added = await store.addWorktree(
      mode() === "new" ? { path: target, newBranch: name().trim(), base: base() } : { path: target, branch: existing() },
    );
    if (!added) return;
    await store.selectWorktree(target);
    const agent = agents().find((candidate) => candidate.id === launch());
    if (agent) {
      const { runInTerminal } = await import("./sidebar.tsx");
      await runInTerminal(target, agent.command);
    }
  }

  return (
    <DialogFrame
      title="New Worktree"
      description="A worktree checks a branch out into its own directory, so an agent can work there while you keep working here."
      actions={<PushButton ui={app} kind="primary" label="Add Worktree" disabled={!valid()} onClick={() => void submit()} />}
    >
      <View style={{ display: "flex", flexDirection: "row", gap: 6 }}>
        <ModeButton label="New branch" active={mode() === "new"} onClick={() => setMode("new")} />
        <ModeButton label="Existing branch" active={mode() === "existing"} onClick={() => setMode("existing")} />
      </View>
      <Show when={mode() === "new"}>
        <Field label="Branch name" error={problem() ?? (duplicate() ? "A branch with this name already exists." : undefined)}>
          <Input
            autoFocus
            value={name()}
            placeholder="agent/fix-login"
            onInput={(event) => setName(event.value ?? "")}
            onSubmit={() => void submit()}
            style={app.styles().input}
          />
        </Field>
        <Field label="Based on">
          <Select.Root
            scope="new-worktree-base"
            ariaLabel="Base branch"
            value={base()}
            items={baseOptions()}
            onValueChange={(value) => value && setBase(value)}
            appearance={pickerAppearance(app)}
            style={[app.styles().input, { flexDirection: "row", alignItems: "center", justifyContent: "space-between", gap: 8 }]}
          >
            <Text style={{ fontSize: 13, color: app.theme().text }}>{base()}</Text>
            <Icon name="chevron-down" size={12} color={app.theme().textTertiary} />
          </Select.Root>
        </Field>
      </Show>
      <Show when={mode() === "existing"}>
        <Field label="Branch" hint={availableBranches().length === 0 ? "Every local branch is already checked out somewhere." : undefined}>
          <Select.Root
            scope="new-worktree-existing"
            ariaLabel="Branch"
            value={existing()}
            items={availableBranches()}
            onValueChange={(value) => value && setExisting(value)}
            appearance={pickerAppearance(app)}
            style={[app.styles().input, { flexDirection: "row", alignItems: "center", justifyContent: "space-between", gap: 8 }]}
          >
            <Text style={{ fontSize: 13, color: existing() ? app.theme().text : app.theme().textTertiary }}>{existing() || "Choose a branch"}</Text>
            <Icon name="chevron-down" size={12} color={app.theme().textTertiary} />
          </Select.Root>
        </Field>
      </Show>
      <Field label="Location" hint="Defaults to a sibling folder named after the branch.">
        <Input
          value={effectivePath()}
          placeholder="/path/to/repo-branch"
          onInput={(event) => {
            setPathTouched(true);
            setPath(event.value ?? "");
          }}
          onSubmit={() => void submit()}
          style={[app.styles().input, app.styles().mono]}
        />
      </Field>
      <Show when={agents().length > 0}>
        <Field label="After adding" hint={launch() === "none" ? undefined : "Opens Terminal in the new worktree and starts the agent there."}>
          <View style={{ display: "flex", flexDirection: "row", flexWrap: "wrap", gap: 6 }}>
            <ModeButton label="Just open it" active={launch() === "none"} onClick={() => setLaunch("none")} />
            {agents().map((agent) => (
              <ModeButton label={`Launch ${agent.label}`} active={launch() === agent.id} onClick={() => setLaunch(agent.id)} />
            ))}
          </View>
        </Field>
      </Show>
    </DialogFrame>
  );
}

function StashDialog() {
  const app = useApp();
  const store = app.store;
  const [message, setMessage] = createSignal("");
  const [includeUntracked, setIncludeUntracked] = createSignal(true);

  async function submit(): Promise<void> {
    app.closeDialog();
    await store.stashPush({ ...(message().trim() ? { message: message().trim() } : {}), includeUntracked: includeUntracked() });
  }

  return (
    <DialogFrame
      title="Stash Changes"
      description="Sets the working tree and index aside so you can come back to them later."
      actions={<PushButton ui={app} kind="primary" label="Stash" onClick={() => void submit()} />}
    >
      <Field label="Message (optional)">
        <Input autoFocus value={message()} placeholder="What were you in the middle of?" onInput={(event) => setMessage(event.value ?? "")} onSubmit={() => void submit()} style={app.styles().input} />
      </Field>
      <CheckRow label="Include untracked files" checked={includeUntracked()} onChange={setIncludeUntracked} />
    </DialogFrame>
  );
}

function ModeButton(props: { label: string; active: boolean; onClick: () => void }) {
  const app = useApp();
  return (
    <PushButton
      ui={app}
      label={props.label}
      onClick={props.onClick}
      style={{
        height: 26,
        fontSize: 12,
        ...(props.active
          ? { backgroundColor: app.theme().accentWash, borderColor: app.theme().accent, color: app.theme().accent }
          : {}),
      }}
    />
  );
}

export function CheckRow(props: { label: string; checked: boolean; onChange: (value: boolean) => void; disabled?: boolean }) {
  const app = useApp();
  return (
    <Checkbox.Root
      checked={props.checked}
      disabled={props.disabled ?? false}
      onCheckedChange={(value) => props.onChange(value)}
      style={{
        display: "flex",
        flexDirection: "row",
        alignItems: "center",
        gap: 8,
        height: 24,
        cursor: "default",
        userSelect: "none",
        borderRadius: 4,
        focus: { outline: `2px solid ${app.theme().focusRing}` },
        disabled: { opacity: 0.5 },
      }}
    >
      <Checkbox.Indicator style={checkboxBox(app, props.checked)}>
        <Show when={props.checked}>
          <Icon name="check" size={11} color={app.theme().textOnAccent} />
        </Show>
      </Checkbox.Indicator>
      <Text style={{ fontSize: 12.5, color: app.theme().text }}>{props.label}</Text>
    </Checkbox.Root>
  );
}

export function checkboxBox(app: ReturnType<typeof useApp>, checked: boolean) {
  return {
    display: "flex" as const,
    width: 15,
    height: 15,
    flexShrink: 0,
    alignItems: "center" as const,
    justifyContent: "center" as const,
    borderRadius: 3.5,
    borderWidth: 1,
    borderColor: checked ? app.theme().accent : app.theme().inputBorder,
    backgroundColor: checked ? app.theme().accent : app.theme().input,
    boxShadow: checked ? "none" : "0 0.5px 1px #00000014",
    transition: "background-color 80ms, border-color 80ms",
  };
}

export function pickerAppearance(app: ReturnType<typeof useApp>) {
  return {
    width: 320,
    rowHeight: 26,
    maxVisibleRows: 10,
    fontSize: 12.5,
    radius: 8,
    padding: 10,
    verticalPadding: 4,
    background: app.theme().raised,
    color: app.theme().text,
    highlightBackground: app.theme().accent,
    highlightColor: app.theme().textOnAccent,
    selectedBackground: app.theme().accentWash,
    mutedColor: app.theme().textTertiary,
  };
}

/** Native confirmation sheet; resolves `true` when the destructive button was chosen. */
export async function confirm(
  app: ReturnType<typeof useApp>,
  options: { message: string; detail?: string; confirmLabel: string; destructive?: boolean },
): Promise<boolean> {
  // A destructive choice is never the Return key's default; Cancel keeps Escape.
  const choice = await NativeDialog.showAlertDialog(app.window, {
    level: options.destructive ? "warning" : "info",
    message: options.message,
    ...(options.detail ? { detail: options.detail } : {}),
    buttons: [
      { label: options.confirmLabel, role: options.destructive ? "other" : "default" },
      { label: "Cancel", role: "cancel" },
    ],
  });
  return choice === 0;
}
