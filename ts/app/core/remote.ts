import * as fs from "fs";
import * as path from "path";

export function isSsh(url: string): boolean {
  return url.startsWith("ssh://") || (url.includes("@") && url.includes(":"));
}

export function parseSshUrl(url: string): { host: string; user: string; path: string } {
  if (url.startsWith("ssh://")) {
    const match = url.match(/^ssh:\/\/([^@]+)@([^/]+)\/(.+)$/);
    if (!match) throw new Error(`Invalid SSH URL: ${url}`);
    return { user: match[1], host: match[2], path: match[3] };
  } else {
    const match = url.match(/^([^@]+)@([^:]+):(.+)$/);
    if (!match) throw new Error(`Invalid SSH URL: ${url}`);
    return { user: match[1], host: match[2], path: match[3] };
  }
}

// ─── Remote Config ──────────────────────────────────────────

const configPath = ".git/config";

interface RemoteConfig {
  [name: string]: { url: string; fetch: string };
}

function readConfig(): RemoteConfig {
  const remotes: RemoteConfig = {};
  if (!fs.existsSync(configPath)) return remotes;

  const content = fs.readFileSync(configPath, "utf-8");
  const lines = content.split("\n");
  let currentRemote = "";

  for (const line of lines) {
    const sectionMatch = line.match(/^\[remote "(.+)"\]$/);
    if (sectionMatch) {
      currentRemote = sectionMatch[1];
      remotes[currentRemote] = { url: "", fetch: "" };
    } else if (currentRemote && line.trim().startsWith("url = ")) {
      remotes[currentRemote].url = line.trim().slice(6);
    } else if (currentRemote && line.trim().startsWith("fetch = ")) {
      remotes[currentRemote].fetch = line.trim().slice(8);
    } else if (line.startsWith("[")) {
      currentRemote = "";
    }
  }
  return remotes;
}

function writeConfig(remotes: RemoteConfig) {
  let content = "";
  // Preserve non-remote sections
  if (fs.existsSync(configPath)) {
    const existing = fs.readFileSync(configPath, "utf-8");
    const lines = existing.split("\n");
    let skip = false;
    for (const line of lines) {
      if (line.match(/^\[remote "/)) { skip = true; continue; }
      if (skip && line.startsWith("[")) { skip = false; }
      if (skip) continue;
      content += line + "\n";
    }
  }

  for (const [name, config] of Object.entries(remotes)) {
    content += `[remote "${name}"]\n`;
    content += `\turl = ${config.url}\n`;
    content += `\tfetch = ${config.fetch}\n`;
  }

  fs.writeFileSync(configPath, content);
}

export function addRemote(name: string, url: string) {
  const remotes = readConfig();
  if (remotes[name]) throw new Error(`fatal: remote ${name} already exists.`);
  remotes[name] = { url, fetch: `+refs/heads/*:refs/remotes/${name}/*` };
  writeConfig(remotes);
}

export function removeRemote(name: string) {
  const remotes = readConfig();
  if (!remotes[name]) throw new Error(`fatal: No such remote: '${name}'`);
  delete remotes[name];
  writeConfig(remotes);
}

export function listRemotes(verbose: boolean): string[] {
  const remotes = readConfig();
  const result: string[] = [];
  for (const [name, config] of Object.entries(remotes)) {
    if (verbose) {
      result.push(`${name}\t${config.url} (fetch)`);
      result.push(`${name}\t${config.url} (push)`);
    } else {
      result.push(name);
    }
  }
  return result;
}

export function getRemoteUrl(name: string): string {
  const remotes = readConfig();
  if (!remotes[name]) throw new Error(`fatal: '${name}' does not appear to be a git repository`);
  return remotes[name].url;
}
