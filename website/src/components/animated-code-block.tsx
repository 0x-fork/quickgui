import { useEffect, useState } from "react";
import { ShikiMagicMoveRenderer } from "@shikijs/magic-move/react";
import type { KeyedTokensInfo } from "@shikijs/magic-move/types";
import type { FrontendTokens } from "../lib/snippets";
import type { DocsFrontend } from "../lib/docs";
import { syncSimilarTokenKeys } from "../lib/code-morph";

const MOVE_OPTIONS = {
  duration: 550,
  stagger: 0.35,
  delayMove: 0.1,
  delayEnter: 0.3,
  delayLeave: 0,
  easing: "cubic-bezier(0.22, 1, 0.36, 1)",
  animateContainer: false,
  containerStyle: false,
};

export function AnimatedCodeBlock({
  tokens,
  frontend,
}: {
  tokens: FrontendTokens;
  frontend: DocsFrontend;
}) {
  const [animate, setAnimate] = useState(false);
  const selected = tokens[frontend];
  const [transition, setTransition] = useState<{
    input: KeyedTokensInfo;
    current: KeyedTokensInfo;
    previous?: KeyedTokensInfo;
  }>(() => ({ input: selected, current: selected }));
  const lineCount = Math.max(...Object.values(tokens).map((item) => item.code.split("\n").length));

  if (transition.input !== selected) {
    const { from, to } = syncSimilarTokenKeys(transition.current, selected);
    setTransition({ input: selected, current: to, previous: from });
  }

  useEffect(() => {
    const preference = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = () => setAnimate(!preference.matches);
    update();
    preference.addEventListener("change", update);
    return () => preference.removeEventListener("change", update);
  }, []);

  return (
    <div
      className="code-morph slim-scroll overflow-auto font-mono text-[12px] sm:text-[13px]"
      style={{ height: `calc(${lineCount} * 1.7em + 3rem)` }}
    >
      <ShikiMagicMoveRenderer
        tokens={transition.current}
        previous={transition.previous}
        animate={animate}
        options={MOVE_OPTIONS}
      />
    </div>
  );
}
