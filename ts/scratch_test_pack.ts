import { createPackfile, parsePackfile } from "./app/packfile.js";
import { resolveHead, readObject } from "./app/git-helpers.js";
import * as fs from "fs";

// This script must be run inside a git repo initialized by mygit
const head = resolveHead();
if (!head) {
  console.log("No HEAD found. Run ./mygit init and commit something first.");
  process.exit(1);
}

console.log("Creating packfile for HEAD:", head);
const pack = createPackfile([head]);
console.log("Packfile created, length:", pack.length);

console.log("Attempting to parse the created packfile...");
const objects = parsePackfile(pack);
console.log("Parsed objects count:", objects.size);
if (objects.has(head)) {
  console.log("SUCCESS: Packfile contains HEAD commit.");
} else {
  console.log("FAILURE: Packfile does not contain HEAD commit.");
}
