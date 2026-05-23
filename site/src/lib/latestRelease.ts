// Build-time fetch of the latest GitHub release. Cached at module scope so
// multiple component invocations share a single HTTP call per build.
//
// Falls back to safe defaults (releases/latest redirect URL, "latest"
// version label) if the fetch fails — offline builds, rate-limited
// networks. Never throws.

type Asset = { name: string; browser_download_url: string; size: number };
type Release = { tag_name: string; assets: Asset[] };

export interface LatestRelease {
  /** e.g. "v0.2.0" — null if fetch failed. */
  tag: string | null;
  /** e.g. "0.2.0" — null if fetch failed. */
  version: string | null;
  /** Direct DMG download URL, or releases/latest page as fallback. */
  dmgUrl: string;
  /** Rounded to 1 decimal place; null if fetch failed. */
  dmgSizeMB: number | null;
}

const FALLBACK: LatestRelease = {
  tag: null,
  version: null,
  dmgUrl: "https://github.com/reyemtech/stint/releases/latest",
  dmgSizeMB: null,
};

let cached: LatestRelease | null = null;

export async function getLatestRelease(): Promise<LatestRelease> {
  if (cached) return cached;
  try {
    const res = await fetch(
      "https://api.github.com/repos/reyemtech/stint/releases/latest",
      { headers: { Accept: "application/vnd.github+json" } },
    );
    if (!res.ok) {
      cached = FALLBACK;
      return cached;
    }
    const release = (await res.json()) as Release;
    const dmg = release.assets.find((a) => a.name.endsWith(".dmg"));
    if (!dmg) {
      cached = FALLBACK;
      return cached;
    }
    cached = {
      tag: release.tag_name,
      version: release.tag_name.replace(/^v/, ""),
      dmgUrl: dmg.browser_download_url,
      dmgSizeMB: Math.round((dmg.size / 1024 / 1024) * 10) / 10,
    };
    return cached;
  } catch {
    cached = FALLBACK;
    return cached;
  }
}
