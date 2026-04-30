import * as fs from "fs";
import * as path from "path";
import { IndexManager } from "../core/index.js";
import { IgnoreManager } from "../core/ignore.js";
import { createBlob } from "../utils/helpers.js";

export function cmdAdd(paths: string[], isForce: boolean) {
  if (paths.length === 0) throw new Error("Nothing specified, nothing added.");
  
  const idx = new IndexManager();
  const ignore = new IgnoreManager();
  
  const addPath = (p: string) => {
    const stats = fs.statSync(p);
    if (stats.isDirectory()) {
      const items = fs.readdirSync(p);
      for (const item of items) {
        const full = path.join(p, item).replace(/\\/g, "/");
        if (item === ".git") continue;
        if (!isForce && ignore.isIgnored(full)) continue;
        addPath(full);
      }
    } else {
      if (!isForce && ignore.isIgnored(p)) return;
      const sha = createBlob(p);
      idx.addEntry(p, sha, stats);
    }
  };
  
  for (const p of paths) {
    if (!fs.existsSync(p)) throw new Error(`fatal: pathspec '${p}' did not match any files`);
    addPath(p);
  }
  idx.writeIndex();
}
