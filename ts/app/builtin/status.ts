import * as fs from "fs";
import { resolveHead, currentBranch, readTreeFlat, getCommitTree } from "../utils/helpers.js";
import { IndexManager } from "../core/index.js";
import { IgnoreManager } from "../core/ignore.js";
import * as crypto from "crypto";
import * as path from "path";

export function cmdStatus() {
  const headSha = resolveHead();
  const branch = currentBranch() || (headSha ? "detached HEAD" : "main");
  console.log(`On branch ${branch}`);
  
  const idx = new IndexManager();
  const staged = idx.entries;
  
  const headFiles = headSha ? readTreeFlat(getCommitTree(headSha)) : new Map<string, string>();
  const stagedNew: string[] = [];
  const stagedModified: string[] = [];
  const stagedDeleted: string[] = [];
  
  for (const entry of staged) {
    if (!headFiles.has(entry.name)) {
      stagedNew.push(entry.name);
    } else if (headFiles.get(entry.name) !== entry.sha1.toString("hex")) {
      stagedModified.push(entry.name);
    }
  }
  for (const [name] of headFiles) {
    if (!staged.some(e => e.name === name)) stagedDeleted.push(name);
  }
  
  if (stagedNew.length || stagedModified.length || stagedDeleted.length) {
    console.log("Changes to be committed:");
    for (const f of stagedNew) console.log(`\tnew file:   ${f}`);
    for (const f of stagedModified) console.log(`\tmodified:   ${f}`);
    for (const f of stagedDeleted) console.log(`\tdeleted:    ${f}`);
    console.log("");
  }
  
  const unstagedModified: string[] = [];
  const unstagedDeleted: string[] = [];
  const untracked: string[] = [];
  const ignore = new IgnoreManager();
  
  const walk = (dir: string) => {
    const items = fs.readdirSync(dir, { withFileTypes: true });
    for (const item of items) {
      const fullPath = path.join(dir, item.name).replace(/\\/g, "/");
      const relPath = fullPath.startsWith("./") ? fullPath.slice(2) : fullPath;
      if (item.name === ".git") continue;
      if (ignore.isIgnored(relPath)) continue;
      
      if (item.isDirectory()) {
        walk(fullPath);
      } else {
        const entry = staged.find(e => e.name === relPath);
        if (entry) {
          const content = fs.readFileSync(fullPath);
          const sha = crypto.createHash("sha1").update(`blob ${content.length}\0`).update(content).digest("hex");
          if (sha !== entry.sha1.toString("hex")) {
            unstagedModified.push(relPath);
          }
        } else if (!headFiles.has(relPath)) {
          untracked.push(relPath);
        }
      }
    }
  };
  walk(".");
  
  for (const entry of staged) {
    if (!fs.existsSync(entry.name)) {
      if (!unstagedDeleted.includes(entry.name)) unstagedDeleted.push(entry.name);
    }
  }
  
  if (unstagedModified.length || unstagedDeleted.length) {
    console.log("Changes not staged for commit:");
    for (const f of unstagedModified) console.log(`\tmodified:   ${f}`);
    for (const f of unstagedDeleted) console.log(`\tdeleted:    ${f}`);
    console.log("");
  }
  
  if (untracked.length) {
    console.log("Untracked files:");
    for (const f of untracked) console.log(`\t${f}`);
    console.log("");
  }
  
  if (!stagedNew.length && !stagedModified.length && !stagedDeleted.length && !unstagedModified.length && !unstagedDeleted.length && !untracked.length) {
    console.log("nothing to commit, working tree clean");
  }
}
