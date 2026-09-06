import { describe, expect, test } from "bun:test";

import {
  Owner,
  batch,
  createContext,
  createEffect,
  createMemo,
  createRenderEffect,
  createRoot,
  createSignal,
  disposeOwner,
  getComputation,
  getOwner,
  onCleanup,
  runWithOwner,
  untrack,
} from "./reactive.ts";

describe("signals and memos", () => {
  test("memos cache and update in dependency order", () => {
    createRoot(() => {
      const [a, setA] = createSignal(1);
      const [b, setB] = createSignal(2);
      let computed = 0;
      const sum = createMemo(() => {
        computed += 1;
        return a() + b();
      });
      const doubled = createMemo(() => sum() * 2);
      expect(sum()).toBe(3);
      expect(doubled()).toBe(6);
      expect(computed).toBe(1);
      setA(5);
      expect(doubled()).toBe(14);
      expect(computed).toBe(2);
      batch(() => {
        setA(1);
        setB(1);
      });
      expect(computed).toBe(3);
      expect(sum()).toBe(2);
    });
  });

  test("a diamond runs the joining memo once per write", () => {
    createRoot(() => {
      const [source, setSource] = createSignal(1);
      const left = createMemo(() => source() + 1);
      const right = createMemo(() => source() * 10);
      let joins = 0;
      const join = createMemo(() => {
        joins += 1;
        return left() + right();
      });
      expect(join()).toBe(12);
      setSource(2);
      expect(join()).toBe(23);
      expect(joins).toBe(2);
    });
  });

  test("equal writes do not notify unless equality is disabled", () => {
    createRoot(() => {
      const [value, setValue] = createSignal(1);
      const [force, setForce] = createSignal(1, { equals: false });
      let runs = 0;
      let forcedRuns = 0;
      createRenderEffect(() => {
        value();
        runs += 1;
      });
      createRenderEffect(() => {
        force();
        forcedRuns += 1;
      });
      setValue(1);
      setForce(1);
      expect(runs).toBe(1);
      expect(forcedRuns).toBe(2);
    });
  });
});

describe("effects", () => {
  test("a stale controller runs before the computations of the scope it controls", () => {
    // A region's content is owned by the component, not by the effect that replaces it. After a
    // few updates the signal's observer order no longer puts that effect first, so without the
    // controller link the content's effect would run against the cleared value.
    const [value, setValue] = createSignal<{ n: number } | undefined>(undefined);
    const seen: number[] = [];
    createRoot(() => {
      const parent = getOwner();
      let content: Owner | undefined;
      createRenderEffect(() => {
        const current = value();
        if (content !== undefined) {
          disposeOwner(content);
          content = undefined;
        }
        if (current === undefined) return;
        const owner = new Owner(parent);
        owner.controller = getComputation();
        content = owner;
        runWithOwner(owner, () => {
          createRenderEffect(() => {
            seen.push(value()!.n);
          });
        });
      });
      for (let index = 0; index < 4; index++) createRenderEffect(() => { void value(); });
    });
    setValue({ n: 1 });
    expect(() => setValue(undefined)).not.toThrow();
    expect(seen).toEqual([1]);
  });

  test("manually disposed child roots leave no retained owners or duplicate cleanups", () => {
    createRoot((dispose) => {
      const owner = getOwner()!;
      let cleanups = 0;
      for (let index = 0; index < 1000; index += 1) {
        createRoot((close) => {
          onCleanup(() => { cleanups += 1; });
          close();
        });
      }
      expect(owner.owned ?? []).toHaveLength(0);
      expect(cleanups).toBe(1000);
      dispose();
      expect(cleanups).toBe(1000);
    });
  });
  test("effects run after the batch and re-run once for several writes", () => {
    createRoot(() => {
      const [a, setA] = createSignal(1);
      const [b, setB] = createSignal(1);
      const seen: number[] = [];
      createEffect(() => {
        seen.push(a() + b());
      });
      expect(seen).toEqual([2]);
      batch(() => {
        setA(2);
        setB(2);
      });
      expect(seen).toEqual([2, 4]);
    });
  });

  test("render effects run immediately and untrack skips subscriptions", () => {
    createRoot(() => {
      const [tracked, setTracked] = createSignal(0);
      const [ignored, setIgnored] = createSignal(0);
      let runs = 0;
      createRenderEffect(() => {
        tracked();
        untrack(() => ignored());
        runs += 1;
      });
      expect(runs).toBe(1);
      setIgnored(1);
      expect(runs).toBe(1);
      setTracked(1);
      expect(runs).toBe(2);
    });
  });

  test("nested computations are disposed when their owner re-runs or the root disposes", () => {
    const cleanups: string[] = [];
    let setOuter: (value: number) => void = () => undefined;
    let setInner: (value: number) => void = () => undefined;
    let innerRuns = 0;
    const dispose = createRoot((disposeRoot) => {
      const [outer, writeOuter] = createSignal(0);
      const [inner, writeInner] = createSignal(0);
      setOuter = writeOuter;
      setInner = writeInner;
      createRenderEffect(() => {
        outer();
        onCleanup(() => cleanups.push("outer"));
        createRenderEffect(() => {
          inner();
          innerRuns += 1;
          onCleanup(() => cleanups.push("inner"));
        });
      });
      return disposeRoot;
    });
    expect(innerRuns).toBe(1);
    setOuter(1);
    expect(cleanups).toEqual(["inner", "outer"]);
    expect(innerRuns).toBe(2);
    setInner(1);
    expect(innerRuns).toBe(3);
    // A re-run releases the previous run's cleanups first, as Solid does.
    expect(cleanups).toEqual(["inner", "outer", "inner"]);
    dispose();
    expect(cleanups).toEqual(["inner", "outer", "inner", "inner", "outer"]);
    setInner(2);
    expect(innerRuns).toBe(3);
  });

  test("a memo read ahead of the batch is brought up to date", () => {
    createRoot(() => {
      const [count, setCount] = createSignal(1);
      const double = createMemo(() => count() * 2);
      let observed = 0;
      createEffect(() => {
        observed = double();
      });
      batch(() => {
        setCount(2);
        expect(double()).toBe(4);
      });
      expect(observed).toBe(4);
    });
  });
});

describe("context", () => {
  test("scopes created by provide are disposed with the computation that created them", () => {
    const context = createContext(0);
    const [value, setValue] = createSignal(0);
    const [page, setPage] = createSignal(0);
    let runs = 0;
    createRoot(() => {
      createRenderEffect(() => {
        page();
        context.provide(1, () => {
          createRenderEffect(() => {
            value();
            runs += 1;
          });
        });
      });
    });
    expect(runs).toBe(1);
    setValue(1);
    expect(runs).toBe(2);
    setPage(1);
    expect(runs).toBe(3);
    // Only the effect of the current page runs; the previous page's effect was disposed.
    setValue(2);
    expect(runs).toBe(4);
  });

  test("provided values are visible to descendants and fall back to the default", () => {
    createRoot(() => {
      const Theme = createContext("light");
      let inner = "";
      let outer = "";
      Theme.provide("dark", () => {
        createRenderEffect(() => {
          inner = Theme.use();
        });
      });
      createRenderEffect(() => {
        outer = Theme.use();
      });
      expect(inner).toBe("dark");
      expect(outer).toBe("light");
    });
  });
});
