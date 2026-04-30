import * as fs from "fs";
import * as zlib from "zlib";
import { resolveHead } from "../utils/helpers.js";

export function cmdLog() {
  let commitSha = resolveHead() || "";
  
  if (!commitSha) {
    console.log("fatal: your current branch 'main' does not have any commits yet");
    return;
  }

  while (commitSha) {
    const dir = commitSha.slice(0, 2);
    const file = commitSha.slice(2);
    const objPath = `.git/objects/${dir}/${file}`;

    if (!fs.existsSync(objPath)) break;

    const compressed = fs.readFileSync(objPath);
    const decompressed = zlib.inflateSync(compressed);
    const nullIndex = decompressed.indexOf(0);
    const content = decompressed.subarray(nullIndex + 1).toString("utf-8");

    const lines = content.split("\n");
    let author = "";
    let parentSha = "";
    let messageLines: string[] = [];
    let inMessage = false;

    for (const line of lines) {
      if (inMessage) {
        messageLines.push(line);
      } else if (line === "") {
        inMessage = true;
      } else if (line.startsWith("parent ")) {
        parentSha = line.slice(7);
      } else if (line.startsWith("author ")) {
        author = line.slice(7);
      }
    }

    const authorMatch = author.match(/^(.+) <(.+)> (\d+) ([+-]\d{4})$/);
    let dateStr = "";
    if (authorMatch) {
      const ts = parseInt(authorMatch[3]) * 1000;
      dateStr = new Date(ts).toUTCString();
    }

    const message = messageLines.join("\n").trim();

    console.log(`\x1b[33mcommit ${commitSha}\x1b[0m`);
    if (authorMatch) {
      console.log(`Author: ${authorMatch[1]} <${authorMatch[2]}>`);
      console.log(`Date:   ${dateStr}`);
    }
    console.log("");
    console.log(`    ${message}`);
    console.log("");

    commitSha = parentSha;
  }
}
