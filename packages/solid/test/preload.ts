import { plugin } from "bun";
import { quickguiSolidPlugin } from "../../cli/src/typescript-compiler.ts";
import { mock } from "bun:test";
import { fakeBinding, setEventDispatcher } from "../../native/test/fake-binding.ts";

mock.module("../../native/src/binding.ts", () => fakeBinding);
plugin(quickguiSolidPlugin({ projectRoot: import.meta.dir }));
const { app } = await import("../../native/src/index.ts");
setEventDispatcher(() => app.dispatchEvents());
