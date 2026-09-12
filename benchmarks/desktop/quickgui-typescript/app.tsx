import { For } from "solid-js";
import { app, Window, PropertyCode } from "@quickgui/native";
import {
  Button,
  Text,
  TextInput,
  View,
  createRenderer,
  type NativeProps,
  type NativeStyle,
} from "@quickgui/solid";
import data from "./issues.generated.json";
import { workload } from "../workload.ts";
import { createTracker } from "./model.ts";

const ink = "#20242c",
  muted = "#737c8c",
  line = "#e2e5eb",
  accent = "#285bd4";
const column: NativeStyle = { display: "flex", flexDirection: "column", minHeight: 0, minWidth: 0 };
const row: NativeStyle = { display: "flex", alignItems: "center", minWidth: 0 };
const Caption = (props: NativeProps) => (
  <Text fontSize={12} lineHeight={18} textColor={muted} {...props} />
);
const Control = (props: NativeProps) => (
  <Button
    style={row}
    justifyContent="center"
    height={32}
    paddingLeft={12}
    paddingRight={12}
    borderWidth={1}
    borderColor="#dce0e7"
    borderRadius={6}
    bg="#ffffff"
    fontSize={12}
    opacity={props.disabled ? 0.4 : 1}
    {...props}
  />
);
const Property = (props: { label: string; value: string }) => (
  <View style={row} justifyContent="space-between">
    <Caption>{props.label}</Caption>
    <Text fontSize={12} fontWeight={500}>
      {props.value}
    </Text>
  </View>
);

function IssueTracker() {
  const state = createTracker(data);
  return (
    <View
      style={row}
      width="100%"
      height="100%"
      alignItems="stretch"
      textColor={ink}
      fontSize={14}
      bg="#ffffff"
    >
      <View
        style={column}
        width={176}
        flexShrink={0}
        paddingTop={24}
        paddingBottom={24}
        paddingLeft={12}
        paddingRight={12}
        gap={8}
        bg="#f4f5f7"
        borderRightWidth={1}
        borderColor={line}
      >
        <Text fontSize={22} fontWeight={700} paddingLeft={12} marginBottom={6}>
          Orbit
        </Text>
        <Caption paddingLeft={12} marginBottom={24}>
          Product workspace
        </Caption>
        <For each={["All issues", "Open", "Completed"]}>
          {(name) => (
            <Button
              style={row}
              height={42}
              paddingLeft={12}
              justifyContent="flex-start"
              borderWidth={0}
              borderRadius={7}
              fontWeight={state.filter() === name ? 600 : 400}
              bg={state.filter() === name ? "#e4ebfb" : "#0000"}
              textColor={state.filter() === name ? accent : ink}
              onClick={() => state.setFilter(name)}
            >
              {name}
            </Button>
          )}
        </For>
        <View flexGrow={1} />
        <Caption padding={12}>{"September cycle\n4 projects · 5 teammates"}</Caption>
      </View>
      <View style={column} flexGrow={1} flexBasis={0}>
        <View
          style={row}
          height={94}
          flexShrink={0}
          paddingLeft={24}
          paddingRight={24}
          justifyContent="space-between"
          borderBottomWidth={1}
          borderColor={line}
        >
          <View style={column} gap={6}>
            <Text fontSize={24} fontWeight={700}>
              Issue inbox
            </Text>
            <Caption>
              {`${state.issues.length - state.completed()} open · ${state.completed()} completed`}
            </Caption>
          </View>
          <TextInput
            value={state.query()}
            placeholder="Search issues, projects, people"
            accessibilityLabel="Search issues"
            onInput={(event) => state.search(event.value ?? "")}
            width={280}
            height={38}
            paddingLeft={12}
            paddingRight={12}
            bg="#f8f9fb"
            borderWidth={1}
            borderColor="#dce0e7"
            borderRadius={7}
          />
        </View>
        <View style={row} flexGrow={1} flexBasis={0} minHeight={0} alignItems="stretch">
          <View style={column} flexGrow={1} flexBasis={0}>
            <View
              style={row}
              height={48}
              flexShrink={0}
              paddingLeft={20}
              paddingRight={20}
              justifyContent="space-between"
              bg="#fafbfc"
              borderBottomWidth={1}
              borderColor={line}
            >
              <Caption>{`${state.matching().length} issues`}</Caption>
              <Caption>Updated this week</Caption>
            </View>
            <For each={[`${state.query()}|${state.filter()}|${state.page()}`]}>
              {() => (
                <View style={column} flexGrow={1} flexBasis={0} overflowY="scroll">
                  <For
                    each={state.visible()}
                    fallback={<Caption padding={20}>No matching issues</Caption>}
                  >
                    {(index) => {
                      const issue = state.issues[index]!;
                      return (
                        <Button
                          style={column}
                          accessibilityLabel={issue.id}
                          height={68}
                          flexShrink={0}
                          justifyContent="center"
                          alignItems="stretch"
                          gap={8}
                          paddingLeft={20}
                          paddingRight={20}
                          borderWidth={0}
                          borderBottomWidth={1}
                          borderRadius={0}
                          borderColor="#edf0f4"
                          bg={state.selected() === index ? "#edf3ff" : "#ffffff"}
                          onClick={() => state.setSelected(index)}
                        >
                          <Text
                            fontSize={14}
                            fontWeight={500}
                            whiteSpace="nowrap"
                            textOverflow="ellipsis"
                            overflow="hidden"
                          >
                            {issue.title}
                          </Text>
                          <Text fontSize={11} textColor={muted} whiteSpace="nowrap">
                            {`${issue.id}  ·  ${issue.project}  ·  ${issue.status()}  ·  ${issue.owner}`}
                          </Text>
                        </Button>
                      );
                    }}
                  </For>
                </View>
              )}
            </For>
            <View
              style={row}
              height={58}
              flexShrink={0}
              justifyContent="space-between"
              paddingLeft={20}
              paddingRight={20}
              borderTopWidth={1}
              borderColor={line}
            >
              <Caption>{`Page ${state.page() + 1} of ${state.pages()}`}</Caption>
              <View style={row} gap={8}>
                <Control disabled={state.page() === 0} onClick={() => state.previous()}>
                  Previous
                </Control>
                <Control disabled={state.page() + 1 >= state.pages()} onClick={() => state.next()}>
                  Next
                </Control>
              </View>
            </View>
          </View>
          <View
            style={column}
            width={350}
            flexShrink={0}
            overflowY="auto"
            padding={24}
            borderLeftWidth={1}
            borderColor={line}
          >
            <View style={column} flexShrink={0}>
              <Caption>{`${state.current().id} / ${state.current().project}`}</Caption>
              <Text fontSize={21} lineHeight={28} fontWeight={700} marginTop={14} marginBottom={22}>
                {state.current().title}
              </Text>
              <View style={column} gap={12} marginBottom={24}>
                <Property label="Status" value={state.current().status()} />
                <Property label="Assignee" value={state.current().owner} />
                <Property label="Priority" value={state.current().priority} />
              </View>
              <Text fontSize={13} lineHeight={20} marginBottom={22}>
                {state.current().description}
              </Text>
              <Text fontSize={12} fontWeight={600} marginBottom={8}>
                Working notes
              </Text>
              <TextInput
                multiline
                value={state.current().notes()}
                accessibilityLabel="Working notes"
                onInput={(event) => state.current().setNotes(event.value ?? "")}
                height={100}
                flexShrink={0}
                padding={10}
                fontSize={13}
                lineHeight={19}
                borderWidth={1}
                borderColor="#dce0e7"
                borderRadius={7}
              />
              <Button
                style={row}
                height={36}
                flexShrink={0}
                justifyContent="center"
                borderWidth={0}
                borderRadius={7}
                marginTop={16}
                bg={accent}
                textColor="#ffffff"
                fontWeight={500}
                onClick={() => state.complete()}
              >
                {state.current().status() === "Done" ? "Reopen issue" : "Mark complete"}
              </Button>
              <Text fontSize={11} textColor={muted} marginTop={10}>
                Changes are kept for this session.
              </Text>
            </View>
          </View>
        </View>
      </View>
    </View>
  );
}

async function openWindow() {
  const check = process.argv.includes("--check-workload");
  const window = new Window({
    title: "Loading issue tracker",
    width: workload.width,
    height: workload.height,
    background: "#ffffff",
    appearance: "light",
    visible: !check,
    focus: !check,
    renderer: createRenderer(IssueTracker),
  });
  const state = await window.getState();
  const rows = [...window.nodes.values()].filter((node) =>
    String(node.properties.get(PropertyCode.AccessibilityLabel) ?? "").startsWith("APP-"),
  );
  if (
    data.length !== workload.records ||
    rows.length !== workload.pageSize ||
    state.viewportSize.width !== workload.width ||
    state.viewportSize.height !== workload.height
  )
    throw new Error("Issue tracker did not mount its complete benchmark workload");
  await window.setTitle(workload.readyTitle);
  if (check) {
    console.log(JSON.stringify({ records: data.length, mountedRows: rows.length, width: state.viewportSize.width, height: state.viewportSize.height }));
    await app.exit();
  }
}
app.on("reopen", ({ hasVisibleWindows }) => {
  if (!hasVisibleWindows) void openWindow();
});
await app.whenReady();
await openWindow();
