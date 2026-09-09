import { createIssues } from "../workload.ts";

const path = new URL("./issues.generated.json", import.meta.url);
const contents = JSON.stringify(createIssues(), null, 2);
if (!(await Bun.file(path).exists()) || (await Bun.file(path).text()) !== contents)
  await Bun.write(path, contents);
