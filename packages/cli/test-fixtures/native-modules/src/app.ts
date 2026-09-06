// Exercises every value shape of the echo module from a compiled application; the CLI test
// compares this program's output line by line.
import { app } from "@quickgui/native";

import * as echo from "../modules/echo/index.ts";

await app.whenReady();

const lines: string[] = [];
lines.push("add=" + String(echo.add(2, 40)));
lines.push("scale=" + String(echo.scale(1.5, 2)));
lines.push("negate=" + String(echo.negate(true)));
echo.nothing();
lines.push("repeat=" + echo.repeat("ab", 3));
lines.push("lengthZ=" + String(echo.lengthZ("héllo")));
lines.push("upper=" + echo.upper("shout"));
const reversed = echo.reverseBytes(new Uint8Array([1, 2, 3]));
lines.push("reverseBytes=" + String(reversed[0]) + "," + String(reversed[1]) + "," + String(reversed[2]));
lines.push("parseHunk=" + JSON.stringify(echo.parseHunk(" keep\n-old\n+new")));
lines.push(
  "countLines=" +
    String(echo.countLines({ heading: "h", lines: [{ kind: "added", text: "x" }], stats: { added: 1, removed: 0 } })),
);
const leaf: echo.Node = { name: "b" };
const branch: echo.Node = { name: "a", children: [leaf] };
lines.push("depth=" + String(echo.depth({ name: "root", children: [branch] })));
lines.push("area=" + String(echo.area({ square: { side: 3 } })));
lines.push("describe=" + JSON.stringify(echo.describe({ none: {} })));
lines.push("pair=" + JSON.stringify(echo.pair(7, "seven")));
lines.push("maybeNull=" + String(echo.maybe(null)));
lines.push("maybe=" + String(echo.maybe(4)));
lines.push("sumAll=" + String(echo.sumAll([1, 2, 3.5])));
lines.push("echoAny=" + JSON.stringify(echo.echoAny(JSON.parse('{"a":[1,"x",null]}') as unknown)));
try {
  echo.fail(1);
  lines.push("fail=none");
} catch (error) {
  lines.push("fail=" + (error as Error).message);
}
try {
  echo.tooBig();
  lines.push("tooBig=none");
} catch (error) {
  lines.push("tooBig=" + (error as Error).message);
}
try {
  echo.add(1.5, 1);
  lines.push("notInteger=none");
} catch (error) {
  lines.push("notInteger=" + (error as Error).message);
}
lines.push("slowSquareAsync=" + String(await echo.slowSquareAsync(100000)));
const addLater = echo.addAsync(1, 2);
const repeatLater = echo.repeatAsync("x", 2);
lines.push("addAsync=" + String(await addLater));
lines.push("repeatAsync=" + (await repeatLater));
try {
  await echo.failAsync(2);
  lines.push("failAsync=none");
} catch (error) {
  lines.push("failAsync=" + (error as Error).message);
}
console.log(lines.join("\n"));
await app.exit(0);
