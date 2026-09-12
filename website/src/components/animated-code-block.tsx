import { useEffect, useMemo, useState } from "react";
import { ShikiMagicMovePrecompiled } from "@shikijs/magic-move/react";
import type { FrontendTokens } from "../lib/snippets";
import { DOCS_FRONTENDS, type DocsFrontend } from "../lib/docs";

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
  const steps = useMemo(() => DOCS_FRONTENDS.map((value) => tokens[value]), [tokens]);
  const lineCount = Math.max(...steps.map((item) => item.code.split("\n").length));

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
      <ShikiMagicMovePrecompiled
        steps={steps}
        step={DOCS_FRONTENDS.indexOf(frontend)}
        animate={animate}
        options={MOVE_OPTIONS}
      />
    </div>
  );
}
