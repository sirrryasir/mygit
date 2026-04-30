import * as builtin from "./builtin/index.js";

const args = process.argv.slice(2);
const command = args[0];

switch (command) {
  case "init":
    builtin.cmdInit();
    break;
  case "cat-file":
    if (args[1] === "-p") builtin.cmdCatFile(args[2]);
    break;
  case "hash-object":
    const write = args.includes("-w");
    const file = write ? args[args.indexOf("-w") + 1] : args[1];
    if (file) builtin.cmdHashObject(file, write);
    break;
  case "ls-tree":
    const nameOnly = args.includes("--name-only");
    const sha = nameOnly ? args[args.indexOf("--name-only") + 1] : args[1];
    if (sha) builtin.cmdLsTree(sha, nameOnly);
    break;
  case "add":
    const force = args.includes("-f") || args.includes("--force");
    const paths = args.slice(1).filter(p => !p.startsWith("-"));
    builtin.cmdAdd(paths, force);
    break;
  case "status":
    builtin.cmdStatus();
    break;
  case "log":
    builtin.cmdLog();
    break;
  case "-v":
  case "--version":
    console.log("mygit-ts version 0.1.0");
    break;
  default:
    console.log(`Unknown command: ${command}`);
}
