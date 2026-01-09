import * as core from '@actions/core';
import * as github from '@actions/github';
import { Octokit } from '@octokit/rest';
import { generateChangelog, Release, ChangelogConfig } from './changelog';

interface EnvConfig {
  token: string;
  title: string;
  minVersion: string;
  releasesPerPage: number;
  maxVersionBranchRegex: RegExp;
  changelogPath: string;
}

function getEnvConfig(): EnvConfig {
  return {
    token: process.env.GITHUB_TOKEN || '',
    title: process.env.CHANGELOG_TITLE || 'Changelog',
    minVersion: process.env.MIN_VERSION || '0.2.0',
    releasesPerPage: parseInt(process.env.RELEASES_PER_PAGE || '100', 10),
    maxVersionBranchRegex: new RegExp(process.env.MAX_VERSION_BRANCH_REGEX || '^\\d+\\.\\d+$'),
    changelogPath: process.env.CHANGELOG_PATH || 'CHANGELOG.md',
  };
}

async function run(): Promise<void> {
  try {
    const config = getEnvConfig();

    if (!config.token) {
      throw new Error('GITHUB_TOKEN is required');
    }

    const octokit = new Octokit({ auth: config.token });

    const context = github.context;
    const { owner, repo } = context.repo;
    const serverUrl = process.env.GITHUB_SERVER_URL || 'https://github.com';
    const baseRepoUrl = `${serverUrl}/${owner}/${repo}`;
    const refName = process.env.GITHUB_REF_NAME || '';
    const baseBranch = refName || 'main';

    console.log(`Generating changelog for ${owner}/${repo}`);
    console.log(`Base branch: ${baseBranch}`);

    const maxVersion = config.maxVersionBranchRegex.test(refName) ? refName : null;
    if (maxVersion) {
      console.log(`Max version constraint from branch: ${maxVersion}`);
    }

    console.log(`Fetching releases (up to ${config.releasesPerPage})...`);
    const { data: releasesData } = await octokit.repos.listReleases({
      owner,
      repo,
      per_page: config.releasesPerPage,
    });

    console.log(`Found ${releasesData.length} releases`);

    const releases: Release[] = releasesData.map((r) => ({
      tag_name: r.tag_name,
      body: r.body ?? null,
      published_at: r.published_at,
      draft: r.draft ?? false,
      prerelease: r.prerelease,
    }));

    const changelogConfig: ChangelogConfig = {
      title: config.title,
      minVersion: config.minVersion,
      maxVersion,
      baseRepoUrl,
    };

    const changelog = generateChangelog(releases, changelogConfig);

    console.log('\n--- Generated Changelog ---\n');
    console.log(changelog);

    core.setOutput('changelog', changelog);

    const updateBranch = `update-changelog--branches--${baseBranch}`;
    console.log(`\nTarget branch: ${updateBranch}`);

    console.log(`Getting SHA for base branch: ${baseBranch}`);
    const { data: baseRef } = await octokit.git.getRef({
      owner,
      repo,
      ref: `heads/${baseBranch}`,
    });
    const baseSha = baseRef.object.sha;
    console.log(`Base SHA: ${baseSha}`);

    let branchExists = false;
    try {
      await octokit.git.getRef({
        owner,
        repo,
        ref: `heads/${updateBranch}`,
      });
      branchExists = true;
      console.log(`Branch ${updateBranch} already exists`);
    } catch (error) {
      console.log(`Branch ${updateBranch} does not exist, will create it`);
    }

    let currentContent: string | null = null;
    let currentFileSha: string | null = null;
    try {
      const { data: fileData } = await octokit.repos.getContent({
        owner,
        repo,
        path: config.changelogPath,
        ref: baseBranch,
      });

      if (!Array.isArray(fileData) && 'content' in fileData) {
        currentContent = Buffer.from(fileData.content, 'base64').toString('utf-8');
        currentFileSha = fileData.sha;
      }
    } catch (error) {
      console.log(`${config.changelogPath} does not exist on ${baseBranch}, will create it`);
    }

    if (currentContent === changelog) {
      console.log('\n✓ Changelog is already up to date, no changes needed');
      core.setOutput('pr_url', '');
      core.setOutput('pr_number', '');
      core.setOutput('branch', updateBranch);
      return;
    }

    console.log('\nChangelog content has changed, proceeding with update...');

    if (!branchExists) {
      await octokit.git.createRef({
        owner,
        repo,
        ref: `refs/heads/${updateBranch}`,
        sha: baseSha,
      });
      console.log(`Created branch: ${updateBranch}`);
    } else {
      await octokit.git.updateRef({
        owner,
        repo,
        ref: `heads/${updateBranch}`,
        sha: baseSha,
        force: true,
      });
      console.log(`Reset branch ${updateBranch} to ${baseBranch}`);
    }

    const commitMessage = `chore(${baseBranch}): update changelog`;
    const contentBase64 = Buffer.from(changelog).toString('base64');

    let updateBranchFileSha: string | undefined;
    try {
      const { data: fileData } = await octokit.repos.getContent({
        owner,
        repo,
        path: config.changelogPath,
        ref: updateBranch,
      });

      if (!Array.isArray(fileData) && 'sha' in fileData) {
        updateBranchFileSha = fileData.sha;
      }
    } catch {
      console.log(`${config.changelogPath} does not exist on ${updateBranch}, will create it`);
      updateBranchFileSha = undefined;
    }

    await octokit.repos.createOrUpdateFileContents({
      owner,
      repo,
      path: config.changelogPath,
      message: commitMessage,
      content: contentBase64,
      branch: updateBranch,
      sha: updateBranchFileSha,
    });
    console.log(`Committed changelog to ${updateBranch}`);

    const { data: existingPrs } = await octokit.pulls.list({
      owner,
      repo,
      head: `${owner}:${updateBranch}`,
      base: baseBranch,
      state: 'open',
    });

    let prUrl: string;
    let prNumber: number;

    if (existingPrs.length > 0) {
      const existingPr = existingPrs[0];
      prUrl = existingPr.html_url;
      prNumber = existingPr.number;
      console.log(`\n✓ Updated existing PR #${prNumber}: ${prUrl}`);
    } else {
      const { data: newPr } = await octokit.pulls.create({
        owner,
        repo,
        title: `chore(${baseBranch}): update changelog`,
        head: updateBranch,
        base: baseBranch,
        body: `This PR updates the changelog for the \`${baseBranch}\` branch.\n\n_This PR was automatically generated._`,
      });
      prUrl = newPr.html_url;
      prNumber = newPr.number;
      console.log(`\n✓ Created PR #${prNumber}: ${prUrl}`);
    }

    core.setOutput('pr_url', prUrl);
    core.setOutput('pr_number', prNumber.toString());
    core.setOutput('branch', updateBranch);

    console.log('\n✓ Changelog update complete');
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    core.setFailed(`Action failed: ${errorMessage}`);
    console.error(error);
  }
}

run().then(
  () => {},
  (error) => core.setFailed(error.message)
);
