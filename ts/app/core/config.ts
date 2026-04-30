import * as fs from "fs";
import * as path from "path";
import * as os from "os";

const GLOBAL_CONFIG = path.join(os.homedir(), ".mygitconfig");
const LOCAL_CONFIG = ".git/config";

export function getConfig(key: string): string | undefined {
  // Try local first, then global
  const local = readConfig(LOCAL_CONFIG);
  if (local[key]) return local[key];
  
  const global = readConfig(GLOBAL_CONFIG);
  return global[key];
}

export function setConfig(key: string, value: string, global: boolean = false) {
  const filePath = global ? GLOBAL_CONFIG : LOCAL_CONFIG;
  const config = readConfig(filePath);
  config[key] = value;
  writeConfig(filePath, config);
}

function readConfig(filePath: string): Record<string, string> {
  const result: Record<string, string> = {};
  if (!fs.existsSync(filePath)) return result;
  
  const content = fs.readFileSync(filePath, "utf-8");
  const lines = content.split("\n");
  let section = "";
  
  for (const line of lines) {
    const sMatch = line.match(/^\[(.+)\]$/);
    if (sMatch) {
      section = sMatch[1].replace(/"/g, "");
      continue;
    }
    
    const kvMatch = line.match(/^\t?([^=]+) = (.+)$/);
    if (kvMatch && section) {
      const key = kvMatch[1].trim();
      const val = kvMatch[2].trim();
      result[`${section}.${key}`] = val;
    }
  }
  return result;
}

function writeConfig(filePath: string, config: Record<string, string>) {
  const sections: Record<string, Record<string, string>> = {};
  
  for (const [fullKey, val] of Object.entries(config)) {
    const lastDot = fullKey.lastIndexOf(".");
    const section = fullKey.substring(0, lastDot);
    const key = fullKey.substring(lastDot + 1);
    
    if (!sections[section]) sections[section] = {};
    sections[section][key] = val;
  }
  
  let content = "";
  for (const [section, kvs] of Object.entries(sections)) {
    content += `[${section}]\n`;
    for (const [k, v] of Object.entries(kvs)) {
      content += `\t${k} = ${v}\n`;
    }
  }
  
  fs.writeFileSync(filePath, content);
}
