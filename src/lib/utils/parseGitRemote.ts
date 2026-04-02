/**
 * Parse a GitHub owner/repo from a git remote URL.
 * Handles https://github.com/owner/repo.git and git@github.com:owner/repo.git
 * Returns null if not a GitHub URL.
 */
export function parseGitHubRepo(remoteUrl: string): string | null {
  // HTTPS format
  const httpsMatch = remoteUrl.match(/github\.com\/([^/]+\/[^/]+?)(?:\.git)?$/);
  if (httpsMatch) return httpsMatch[1];

  // SSH format
  const sshMatch = remoteUrl.match(/github\.com:([^/]+\/[^/]+?)(?:\.git)?$/);
  if (sshMatch) return sshMatch[1];

  return null;
}
