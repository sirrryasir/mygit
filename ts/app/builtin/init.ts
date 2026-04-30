import * as fs from "fs";
import * as path from "path";

export function cmdInit() {
  fs.mkdirSync(".git", { recursive: true });
  fs.mkdirSync(".git/objects", { recursive: true });
  fs.mkdirSync(".git/refs", { recursive: true });
  fs.writeFileSync(".git/HEAD", "ref: refs/heads/main\n");
  console.log("Initialized empty Git repository in " + path.join(process.cwd(), ".git/"));
}
