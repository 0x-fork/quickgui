import { createMemo, createSignal, For, onCleanup } from "solid-js";
import { SystemPreferences } from "@quickgui/native";
import { Button, Text, View, type JSX } from "@quickgui/solid";
import { CHARACTER_WIDTH, FONT_SIZE, LINE_HEIGHT, layoutTokens, planMorph } from "./code-morph.ts";
import { filenames, languageLabels, languages, type Snippets, type Language } from "./snippets.ts";

const control = {
  display: "flex",
  alignItems: "center",
  justifyContent: "center",
  height: 34,
  paddingLeft: 14,
  paddingRight: 14,
  borderRadius: 7,
  fontSize: 13,
  color: "#9ca8b8",
  cursor: "pointer",
  userSelect: "none",
  hover: { bg: "#232c3c", color: "#f0f6fc" },
  focus: { outline: "2px solid #79b8ff" },
  outlineOffset: -2,
} satisfies JSX.Style;

export function CodeMorphDemo(props: { snippets: Snippets; reduceMotion: boolean }) {
  const initialLanguage: Language = "go";
  const [language, setLanguage] = createSignal<Language>(initialLanguage);
  const [slow, setSlow] = createSignal(false);
  const [reduceMotion, setReduceMotion] = createSignal(props.reduceMotion);
  const [duration, setDuration] = createSignal(550);
  let current = props.snippets[initialLanguage];
  let target = layoutTokens(current);
  const [tokens, setTokens] = createSignal(target);
  const keys = createMemo(() => [...tokens().keys()]);
  let enterTimer: ReturnType<typeof setTimeout> | undefined;
  let leaveTimer: ReturnType<typeof setTimeout> | undefined;

  function cancelTimers() {
    clearTimeout(enterTimer);
    clearTimeout(leaveTimer);
    enterTimer = leaveTimer = undefined;
  }

  function select(nextLanguage: Language) {
    if (nextLanguage === language()) return;
    cancelTimers();
    const plan = planMorph(current, props.snippets[nextLanguage], tokens());
    current = plan.current;
    target = plan.target;
    setLanguage(nextLanguage);
    const milliseconds = reduceMotion() ? 0 : slow() ? 1600 : 550;
    setDuration(milliseconds);
    if (milliseconds === 0) {
      setTokens(target);
      return;
    }
    setTokens(plan.start);
    // Give new nodes an initial frame before fading them in. These two bounded deadlines
    // only stage entry and remove exits; there is no JavaScript animation/frame loop.
    const enterDelay = Math.max(32, Math.round(milliseconds * 0.25));
    if (plan.entering.length) {
      enterTimer = setTimeout(() => {
        enterTimer = undefined;
        const entered = new Map(tokens());
        for (const key of plan.entering) entered.set(key, plan.target.get(key)!);
        setTokens(entered);
      }, enterDelay);
    }
    leaveTimer = setTimeout(
      () => {
        leaveTimer = undefined;
        setTokens(plan.target);
      },
      enterDelay + milliseconds + 64,
    );
  }

  onCleanup(cancelTimers);
  onCleanup(
    SystemPreferences.onChange((preferences) => {
      setReduceMotion(preferences.reduceMotion === true);
      if (preferences.reduceMotion) {
        cancelTimers();
        setDuration(0);
        setTokens(target);
      }
    }),
  );

  // One fixed canvas for every snippet keeps both scroll axes stable while code moves.
  const lines = languages.map((lang) => props.snippets[lang].code.split("\n"));
  const rowCount = Math.max(...lines.map((source) => source.length));
  const codeWidth = Math.ceil(
    Math.max(...lines.flat().map((line) => line.length)) * CHARACTER_WIDTH,
  );
  const lineNumbers = Array.from({ length: rowCount }, (_, index) => index + 1);

  return (
    <View flex-col size-full padding={28} gap={22} bg="#0d1117" color="#e6edf3">
      <View flex-col gap={7} flex-shrink-0>
        <Text fontSize={11} fontWeight={600} color="#79b8ff">
          QUICKGUI / TYPESCRIPT
        </Text>
        <Text fontSize={30} fontWeight={600}>
          Code, in motion.
        </Text>
        <Text fontSize={14} color="#8b98aa">
          The same idea, in three languages. Follow the pieces as they move.
        </Text>
      </View>

      <View
        flex-col
        flex-1
        minHeight={0}
        borderWidth={1}
        borderColor="#293241"
        borderRadius={12}
        overflow="hidden"
        bg="#101620"
      >
        <View
          flex-row
          items-center
          justify-between
          padding={12}
          borderBottomWidth={1}
          borderColor="#293241"
          flex-shrink-0
        >
          <View flex-row gap={4}>
            <For each={languages}>
              {(item) => (
                <Button
                  ariaLabel={`Show ${languageLabels[item]}`}
                  selected={language() === item}
                  style={control}
                  bg={language() === item ? "#243249" : "transparent"}
                  color={language() === item ? "#9dcbff" : "#9ca8b8"}
                  onClick={() => select(item)}
                >
                  {languageLabels[item]}
                </Button>
              )}
            </For>
          </View>
          <Text fontFamily="Menlo" fontSize={12} color="#6d7c90" paddingRight={8}>
            {filenames[language()]}
          </Text>
        </View>

        <View flex-1 minHeight={0} overflow="auto" padding={24}>
          <View
            position="relative"
            width={codeWidth + 56}
            height={rowCount * LINE_HEIGHT + 8}
            userSelect="none"
          >
            <For each={lineNumbers}>
              {(line) => (
                <Text
                  position="absolute"
                  left={0}
                  top={(line - 1) * LINE_HEIGHT}
                  width={26}
                  fontFamily="Menlo"
                  fontSize={FONT_SIZE}
                  lineHeight={LINE_HEIGHT}
                  textAlign="right"
                  color="#48566a"
                >
                  {line}
                </Text>
              )}
            </For>
            <View
              position="absolute"
              left={48}
              top={0}
              width={codeWidth + 8}
              height={rowCount * LINE_HEIGHT}
            >
              <For each={keys()}>
                {(key) => {
                  // For keys, rather than token objects, preserves native IDs across every move.
                  const token = createMemo(() => tokens().get(key)!);
                  return (
                    <Text
                      position="absolute"
                      left={0}
                      top={0}
                      fontFamily="Menlo"
                      fontSize={FONT_SIZE}
                      lineHeight={LINE_HEIGHT}
                      whiteSpace="nowrap"
                      color={token().color}
                      opacity={token().opacity}
                      transform={`translate(${token().x}px, ${token().y}px)`}
                      transition={{ property: "all", duration: duration(), easing: "ease-out" }}
                    >
                      {token().content}
                    </Text>
                  );
                }}
              </For>
            </View>
          </View>
        </View>

        <View
          flex-row
          items-center
          justify-between
          padding={14}
          borderTopWidth={1}
          borderColor="#293241"
          flex-shrink-0
        >
          <Text fontSize={12} color="#8b98aa">
            {reduceMotion()
              ? "Reduced motion follows your system setting."
              : "Switch languages to watch matching words find their place."}
          </Text>
          <Text fontFamily="Menlo" fontSize={12} color="#6d7c90">
            {props.snippets[language()].code.split("\n").length} lines
          </Text>
        </View>
      </View>

      <View flex-row items-center justify-between flex-shrink-0>
        <Button
          ariaLabel="Toggle slow motion"
          selected={slow()}
          style={control}
          bg={slow() ? "#243249" : "#161d28"}
          onClick={() => setSlow(!slow())}
        >
          {slow() ? "Slow motion: on" : "Slow motion: off"}
        </Button>
        <Button
          ariaLabel="Next language"
          style={{ ...control, hover: { bg: "#a5d0ff", color: "#0d1117" } }}
          bg="#79b8ff"
          color="#0d1117"
          fontWeight={600}
          onClick={() => select(languages[(languages.indexOf(language()) + 1) % languages.length]!)}
        >
          Next language →
        </Button>
      </View>
    </View>
  );
}
