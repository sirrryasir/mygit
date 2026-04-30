import * as fs from "fs";
import * as crypto from "crypto";
import * as zlib from "zlib";
import * as path from "path";

// ─── Object Store ───────────────────────────────────────────

export function readObject(sha: string): { type: string; content: Buffer } {
  const objPath = `.git/objects/${sha.slice(0, 2)}/${sha.slice(2)}`;
  if (!fs.existsSync(objPath)) throw new Error(`fatal: Not a valid object name ${sha}`);
  const compressed = fs.readFileSync(objPath);
  const decompressed = zlib.inflateSync(compressed);
  const nullIdx = decompressed.indexOf(0);
  const header = decompressed.subarray(0, nullIdx).toString("utf-8");
  const [type] = header.split(" ");
  const content = decompressed.subarray(nullIdx + 1);
  return { type, content };
}

export function writeObject(type: string, content: Buffer): string {
  const header = `${type} ${content.length}\0`;
  const store = Buffer.concat([Buffer.from(header), content]);
  const sha = crypto.createHash("sha1").update(store).digest("hex");
  const dir = `.git/objects/${sha.slice(0, 2)}`;
  const filePath = `${dir}/${sha.slice(2)}`;
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
  if (!fs.existsSync(filePath)) {
    fs.writeFileSync(filePath, zlib.deflateSync(store));
  }
  return sha;
}

export function createBlob(filePath: string): string {
  const content = fs.readFileSync(filePath);
  return writeObject("blob", content);
}

// ─── HEAD / Refs ────────────────────────────────────────────

export function resolveHead(): string {
  if (!fs.existsSync(".git/HEAD")) return "";
  const headContent = fs.readFileSync(".git/HEAD", "utf-8").trim();
  if (headContent.startsWith("ref: ")) {
    const refPath = `.git/${headContent.slice(5)}`;
    if (fs.existsSync(refPath)) {
      return fs.readFileSync(refPath, "utf-8").trim();
    }
    return "";
  }
  return headContent;
}

export function currentBranch(): string {
  if (!fs.existsSync(".git/HEAD")) return "";
  const headContent = fs.readFileSync(".git/HEAD", "utf-8").trim();
  if (headContent.startsWith("ref: refs/heads/")) {
    return headContent.slice("ref: refs/heads/".length);
  }
  return "";
}

export function updateRef(refName: string, sha: string) {
  const refPath = `.git/${refName}`;
  const refDir = path.dirname(refPath);
  if (!fs.existsSync(refDir)) fs.mkdirSync(refDir, { recursive: true });
  fs.writeFileSync(refPath, sha + "\n");
}

export function updateHead(sha: string) {
  const headContent = fs.readFileSync(".git/HEAD", "utf-8").trim();
  if (headContent.startsWith("ref: ")) {
    updateRef(headContent.slice(5), sha);
  } else {
    fs.writeFileSync(".git/HEAD", sha + "\n");
  }
}

// ─── Tree Parsing ───────────────────────────────────────────

export function readTreeFlat(treeSha: string, prefix: string = ""): Map<string, string> {
  const files = new Map<string, string>();
  const { content } = readObject(treeSha);
  let offset = 0;

  while (offset < content.length) {
    let spaceIdx = offset;
    while (content[spaceIdx] !== 0x20) spaceIdx++;
    const mode = content.subarray(offset, spaceIdx).toString("utf-8");

    let nameEnd = spaceIdx + 1;
    while (content[nameEnd] !== 0) nameEnd++;
    const name = content.subarray(spaceIdx + 1, nameEnd).toString("utf-8");

    const sha = content.subarray(nameEnd + 1, nameEnd + 21);
    const shaHex = Buffer.from(sha).toString("hex");
    offset = nameEnd + 21;

    const fullPath = prefix ? `${prefix}/${name}` : name;
    if (mode === "40000") {
      const subFiles = readTreeFlat(shaHex, fullPath);
      for (const [k, v] of subFiles) files.set(k, v);
    } else {
      files.set(fullPath, shaHex);
    }
  }
  return files;
}

export function getCommitTree(commitSha: string): string {
  const { content } = readObject(commitSha);
  const text = content.toString("utf-8");
  const match = text.match(/^tree ([a-f0-9]{40})/);
  return match ? match[1] : "";
}

export function getCommitParents(commitSha: string): string[] {
  const { content } = readObject(commitSha);
  const text = content.toString("utf-8");
  const parents: string[] = [];
  for (const line of text.split("\n")) {
    if (line.startsWith("parent ")) parents.push(line.slice(7));
    if (line === "") break;
  }
  return parents;
}

// ─── Commit Creation ────────────────────────────────────────

import { getConfig } from "./config.js";

export function createCommit(treeSha: string, parentShas: string[], message: string): string {
  const timestamp = Math.floor(Date.now() / 1000);
  const timezone = "+0000";
  const authorName = getConfig("user.name") || "Yasir";
  const authorEmail = getConfig("user.email") || "yasir@example.com";

  let commitContent = `tree ${treeSha}\n`;
  for (const parent of parentShas) {
    if (parent) commitContent += `parent ${parent}\n`;
  }
  commitContent += `author ${authorName} <${authorEmail}> ${timestamp} ${timezone}\n`;
  commitContent += `committer ${authorName} <${authorEmail}> ${timestamp} ${timezone}\n\n`;
  commitContent += `${message}\n`;

  return writeObject("commit", Buffer.from(commitContent));
}

// ─── Ancestor Check ─────────────────────────────────────────

export function isAncestor(potentialAncestor: string, commitSha: string): boolean {
  // BFS through parents
  const visited = new Set<string>();
  const queue = [commitSha];
  while (queue.length > 0) {
    const current = queue.shift()!;
    if (current === potentialAncestor) return true;
    if (visited.has(current)) continue;
    visited.add(current);
    const parents = getCommitParents(current);
    queue.push(...parents);
  }
  return false;
}
