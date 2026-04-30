import * as fs from "fs";
import * as path from "path";

export class IgnoreManager {
  private patterns: string[] = [];

  constructor() {
    this.loadPatterns(".gitignore");
    this.loadPatterns(".mygitignore");
    // Standard git ignores
    this.patterns.push(".git/");
  }

  private loadPatterns(file: string) {
    if (fs.existsSync(file)) {
      const content = fs.readFileSync(file, "utf-8");
      const lines = content.split("\n");
      for (let line of lines) {
        line = line.trim();
        if (line && !line.startsWith("#")) {
          this.patterns.push(line);
        }
      }
    }
  }

  public isIgnored(filePath: string): boolean {
    for (const pattern of this.patterns) {
      if (this.match(pattern, filePath)) return true;
    }
    return false;
  }

  private match(pattern: string, filePath: string): boolean {
    // Simple glob to regex conversion
    // handle directory-only patterns (ending in /)
    const isDirPattern = pattern.endsWith("/");
    const cleanPattern = isDirPattern ? pattern.slice(0, -1) : pattern;
    
    // Convert glob stars to regex
    let regexStr = cleanPattern
      .replace(/\./g, "\\.")
      .replace(/\*\*/g, "(.+)")
      .replace(/\*/g, "([^/]+)")
      .replace(/\?/g, "(.)");
      
    if (isDirPattern) {
      regexStr = `^${regexStr}/|^${regexStr}$`;
    } else {
      regexStr = `^${regexStr}$|^${regexStr}/`;
    }
    
    const regex = new RegExp(regexStr);
    
    // Check path parts
    const parts = filePath.split("/");
    let current = "";
    for (const part of parts) {
      current = current ? `${current}/${part}` : part;
      if (regex.test(current)) return true;
      // Also match basename if pattern doesn't have /
      if (!pattern.includes("/") && regex.test(part)) return true;
    }
    
    return false;
  }
}
