import * as semver from 'semver';

/**
 * Release data from GitHub API
 */
export interface Release {
  tag_name: string;
  body: string | null;
  published_at: string | null;
  draft: boolean;
  prerelease: boolean;
}

/**
 * Configuration for changelog generation
 */
export interface ChangelogConfig {
  title: string;
  minVersion: string;
  maxVersion: string | null;
  baseRepoUrl: string;
}

/**
 * Extract YYYY-MM-DD from an ISO date string
 */
export function toDay(iso: string | null | undefined): string {
  return String(iso || '').slice(0, 10);
}

/**
 * Escape special regex characters in a string
 */
function escapeRegExp(s: string): string {
  return String(s).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Parse major.minor from a tag (ignoring patch and prerelease)
 */
export function parseMajorMinor(tag: string): { major: number; minor: number } | null {
  const m = String(tag).trim().match(/^v?(\d+)\.(\d+)(?:\.\d+)?/);
  return m ? { major: Number(m[1]), minor: Number(m[2]) } : null;
}

/**
 * Format a semver object back to string (without leading 'v')
 */
export function formatVersion(parsed: semver.SemVer | null): string | null {
  if (!parsed) return null;
  return parsed.version;
}

/**
 * Compare two semver strings in ascending order
 * Returns negative if a < b, positive if a > b, 0 if equal
 */
export function compareSemverAsc(a: string, b: string): number {
  const parsedA = semver.parse(a);
  const parsedB = semver.parse(b);

  if (!parsedA && !parsedB) return 0;
  if (!parsedA) return -1;
  if (!parsedB) return 1;

  return semver.compare(parsedA, parsedB);
}

/**
 * Compare two semver strings in descending order
 */
export function compareSemverDesc(a: string, b: string): number {
  return -compareSemverAsc(a, b);
}

/**
 * Check if a version is below the minimum version
 */
export function isBelowMinVersion(tag: string, minVersion: string): boolean {
  const parsed = semver.parse(tag);
  const minParsed = semver.parse(minVersion);

  if (!parsed || !minParsed) return false;

  return semver.lt(parsed, minParsed);
}

/**
 * Check if a release should be kept based on max version constraint
 */
export function shouldKeepUnderMax(
  tag: string,
  maxVersion: string | null
): boolean {
  if (!maxVersion) return true;

  const maxMajorMinor = parseMajorMinor(maxVersion);
  if (!maxMajorMinor) return true;

  const tagMajorMinor = parseMajorMinor(tag);
  if (!tagMajorMinor) return false; // if maxVersion is set, drop non-semver tags

  return (
    tagMajorMinor.major < maxMajorMinor.major ||
    (tagMajorMinor.major === maxMajorMinor.major &&
      tagMajorMinor.minor <= maxMajorMinor.minor)
  );
}

/**
 * Build a map from tag to its previous tag (based on semver ordering)
 */
export function buildPrevTagMap(releases: Release[]): Map<string, string | null> {
  const semverList = releases
    .map((r) => ({
      tag: String(r.tag_name || ''),
      parsed: semver.parse(r.tag_name),
    }))
    .filter((x): x is { tag: string; parsed: semver.SemVer } => x.parsed !== null)
    .sort((a, b) => semver.compare(a.parsed, b.parsed));

  const prevByTag = new Map<string, string | null>();
  for (let i = 0; i < semverList.length; i++) {
    prevByTag.set(semverList[i].tag, i > 0 ? semverList[i - 1].tag : null);
  }
  return prevByTag;
}

/**
 * Normalize heading levels in release body markdown
 * - First line becomes H2
 * - All later headings are shifted so minimum is H3
 */
export function normalizeBodyHeadings(input: string | null | undefined): string {
  const text = String(input || '').replace(/\r\n/g, '\n');
  if (!text) return text;

  const lines = text.split('\n');
  if (lines.length === 0) return text;

  const headingMatch = (line: string) => line.match(/^(#{1,6})\s+(.*)$/);

  // First line must be H2
  const first = headingMatch(lines[0]);
  lines[0] = first ? `## ${first[2]}` : `## ${lines[0].trim()}`;

  // If any later heading is H1/H2, shift *all* later headings equally
  // so the minimum later heading becomes H3 (preserves hierarchy)
  let minLaterLevel = Infinity;
  for (let i = 1; i < lines.length; i++) {
    const m = headingMatch(lines[i]);
    if (m) minLaterLevel = Math.min(minLaterLevel, m[1].length);
  }

  const delta = minLaterLevel <= 2 ? 3 - minLaterLevel : 0;
  if (delta > 0) {
    for (let i = 1; i < lines.length; i++) {
      const m = headingMatch(lines[i]);
      if (!m) continue;
      const newLevel = Math.min(6, m[1].length + delta);
      lines[i] = `${'#'.repeat(newLevel)} ${m[2]}`;
    }
  }

  return lines.join('\n');
}

/**
 * Generate a release header with compare link
 */
function releaseHeader(
  versionString: string,
  tag: string,
  date: string,
  prevTag: string | null,
  baseRepoUrl: string
): string {
  const link = prevTag
    ? `${baseRepoUrl}/compare/${prevTag}...${tag}`
    : `${baseRepoUrl}/releases/tag/${tag}`;
  return `## [${versionString}](${link}) (${date})`;
}

/**
 * Ensure a release body has a proper header if missing
 */
export function ensureHeaderIfMissing(
  release: Release,
  prevTagByTag: Map<string, string | null>,
  baseRepoUrl: string
): string {
  const tag = String(release.tag_name || '').trim();
  const parsed = semver.parse(tag);
  const versionString = formatVersion(parsed);

  const rawBody = String(release.body || '');
  const trimmed = rawBody.trim();
  if (!trimmed) return '';

  // If tag isn't semver, just normalize headings
  if (!versionString) return normalizeBodyHeadings(trimmed);

  // If first line already contains the semver (with optional "v"), don't add a header
  const firstLine = rawBody.trimStart().split('\n')[0] || '';
  const semverInFirstLine = new RegExp(`\\bv?${escapeRegExp(versionString)}\\b`).test(
    firstLine
  );
  if (semverInFirstLine) return normalizeBodyHeadings(trimmed);

  const date = toDay(release.published_at);
  const prevTag = prevTagByTag.get(tag) || null;
  const header = releaseHeader(versionString, tag, date, prevTag, baseRepoUrl);
  return normalizeBodyHeadings(`${header}\n\n\n${trimmed}`);
}

/**
 * Filter releases based on version constraints
 */
export function filterReleases(
  releases: Release[],
  minVersion: string,
  maxVersion: string | null
): Release[] {
  return releases
    .filter((r) => !r.draft)
    .filter((r) => !isBelowMinVersion(r.tag_name, minVersion))
    .filter((r) => shouldKeepUnderMax(r.tag_name, maxVersion));
}

/**
 * Sort releases by date (descending), then stable before prerelease, then semver (descending)
 */
export function sortReleases(releases: Release[]): Release[] {
  return releases.slice().sort((a, b) => {
    const dayA = toDay(a.published_at);
    const dayB = toDay(b.published_at);

    // Date descending (day only)
    if (dayA !== dayB) return dayB.localeCompare(dayA);

    // Stable before prerelease
    if (Boolean(a.prerelease) !== Boolean(b.prerelease)) {
      return a.prerelease ? 1 : -1;
    }

    // Semver descending
    return compareSemverDesc(a.tag_name, b.tag_name);
  });
}

/**
 * Generate the full changelog markdown
 */
export function generateChangelog(
  releases: Release[],
  config: ChangelogConfig
): string {
  const { title, minVersion, maxVersion, baseRepoUrl } = config;

  // Filter releases
  const filtered = filterReleases(releases, minVersion, maxVersion);

  // Build prev tag map for compare links
  const prevTagByTag = buildPrevTagMap(filtered);

  // Sort releases
  const sorted = sortReleases(filtered);

  // Render bodies
  const bodies = sorted
    .map((r) => ensureHeaderIfMissing(r, prevTagByTag, baseRepoUrl))
    .filter(Boolean);

  // Build final changelog
  return [`# ${title}`, ...bodies].join('\n\n') + '\n';
}

