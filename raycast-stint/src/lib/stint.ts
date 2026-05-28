import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { getPreferenceValues } from "@raycast/api";

const execFileAsync = promisify(execFile);

interface Preferences {
  stintBin: string;
}

let cachedBinPath: string | null = null;

function resolveBinPath(): string {
  const pref = getPreferenceValues<Preferences>().stintBin?.trim();
  if (pref) return pref;
  if (cachedBinPath) return cachedBinPath;

  const candidates = [
    "/usr/local/bin/stint",
    join(homedir(), ".cargo/bin/stint"),
    "/Applications/Stint.app/Contents/MacOS/stint",
  ];
  for (const path of candidates) {
    if (existsSync(path)) {
      cachedBinPath = path;
      return path;
    }
  }
  throw new Error(
    "stint binary not found. Set the path in Raycast preferences.",
  );
}

export async function stint<T = unknown>(...args: string[]): Promise<T> {
  const bin = resolveBinPath();
  const { stdout } = await execFileAsync(bin, ["--json", ...args], {
    timeout: 10_000,
    maxBuffer: 4 * 1024 * 1024,
  });
  const trimmed = stdout.trim();
  if (!trimmed) return undefined as T;
  return JSON.parse(trimmed) as T;
}
