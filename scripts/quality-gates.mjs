import { execFile } from 'node:child_process';
import { mkdtemp, mkdir, readFile, realpath, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const MAX_BUFFER = 100 * 1024 * 1024;
const SKYLOS_CATEGORIES = ['danger', 'security', 'secrets', 'quality'];

export async function readPolicy(root) {
  const file = path.join(root, '.sonata', 'quality-gates.json');
  const policy = JSON.parse(await readFile(file, 'utf8'));
  if (policy.schema !== 1) throw new Error(`Unsupported quality-gates schema: ${policy.schema}`);
  for (const tool of ['scc', 'skylos']) {
    if (policy[tool]?.enabled && !/^\d+\.\d+\.\d+$/.test(policy[tool].version || '')) {
      throw new Error(`${tool} requires an exact x.y.z version in .sonata/quality-gates.json.`);
    }
  }
  return policy;
}

export async function verifyTool(command, expectedVersion) {
  let output;
  try {
    const result = await run(command, ['--version']);
    output = `${result.stdout}\n${result.stderr}`;
  } catch (error) {
    if (error.code === 'ENOENT') {
      throw new Error(`${command} ${expectedVersion} is enabled but not installed. Run $sonata-setup for installation guidance.`);
    }
    throw error;
  }
  const actual = output.match(/\d+\.\d+\.\d+/)?.[0];
  if (actual !== expectedVersion) {
    throw new Error(`${command} version ${expectedVersion} is required; found ${actual || 'unknown'}. Run $sonata-setup to update the pin or tool.`);
  }
}

export async function resolveBase(root, override, env = process.env) {
  const explicit = override || env.SONATA_BASE_REF;
  if (explicit) {
    await git(root, ['rev-parse', '--verify', explicit]);
    return explicit;
  }
  if (!env.GITHUB_BASE_REF) return 'HEAD';
  for (const candidate of [`origin/${env.GITHUB_BASE_REF}`, env.GITHUB_BASE_REF]) {
    try {
      return (await git(root, ['merge-base', 'HEAD', candidate])).trim();
    } catch {}
  }
  throw new Error(`Cannot resolve GitHub base branch ${env.GITHUB_BASE_REF}. Fetch the base branch or pass --base <ref>.`);
}

export async function changedFiles(root, base) {
  const diff = await git(root, ['diff', '--name-status', '-z', base, '--']);
  const tokens = diff.split('\0').filter(Boolean);
  const changes = [];
  for (let index = 0; index < tokens.length;) {
    const status = tokens[index++];
    if (status.startsWith('R') || status.startsWith('C')) {
      const oldPath = tokens[index++];
      const nextPath = tokens[index++];
      const comparableRename = status.startsWith('R') && path.extname(oldPath) === path.extname(nextPath);
      changes.push({ status: comparableRename ? 'R' : 'A', oldPath, path: nextPath });
    } else {
      changes.push({ status: status[0], path: tokens[index++] });
    }
  }
  const tracked = new Set(changes.map(({ path: file }) => file));
  const untracked = await git(root, ['ls-files', '--others', '--exclude-standard', '-z']);
  for (const file of untracked.split('\0').filter(Boolean)) {
    if (!tracked.has(file)) changes.push({ status: 'A', path: file });
  }
  return changes.filter(({ status }) => status !== 'D');
}

export async function materializeBase(root, base, changes) {
  const tempRoot = await realpath(await mkdtemp(path.join(os.tmpdir(), 'sonata-quality-base-')));
  const baseline = changes.filter(({ status }) => status !== 'A');
  for (const change of baseline) {
    const source = change.oldPath || change.path;
    try {
      const content = await gitBuffer(root, ['show', `${base}:${source}`]);
      const target = safeTarget(tempRoot, change.path);
      await mkdir(path.dirname(target), { recursive: true });
      await writeFile(target, content);
    } catch (error) {
      await rm(tempRoot, { recursive: true, force: true });
      throw new Error(`Cannot read ${source} from ${base}: ${error.message}`);
    }
  }
  return tempRoot;
}

export async function runScc(cwd, files) {
  if (!files.length) return new Map();
  const { stdout } = await run('scc', ['--by-file', '--format', 'json', ...files], { cwd });
  const languages = parseJson(stdout, 'SCC');
  const records = new Map();
  for (const language of languages) {
    for (const file of language.Files || []) {
      records.set(normalizePath(file.Location), {
        language: file.Language || language.Name,
        complexity: Number(file.Complexity || 0)
      });
    }
  }
  return records;
}

export function evaluateScc(current, baseline, changes, thresholds = {}) {
  const failures = [];
  for (const [file, result] of current) {
    const prior = baseline.get(file);
    if (prior && result.complexity > prior.complexity) {
      failures.push(`${file}: ${result.language} complexity increased ${prior.complexity} -> ${result.complexity}`);
      continue;
    }
    if (prior) continue;
    const limit = thresholds[result.language];
    if (!Number.isFinite(limit)) {
      failures.push(`${file}: no SCC ceiling configured for ${result.language}; run $sonata-setup`);
    } else if (result.complexity > limit) {
      failures.push(`${file}: ${result.language} complexity ${result.complexity} exceeds ${limit}`);
    }
  }
  return failures;
}

export async function recommendScc(root) {
  const { stdout } = await run('scc', ['--by-file', '--format', 'json', '.'], { cwd: root });
  const languages = parseJson(stdout, 'SCC');
  const recommendations = {};
  for (const language of languages) {
    const values = (language.Files || []).map((file) => Number(file.Complexity || 0)).sort((a, b) => a - b);
    if (!values.length) continue;
    const percentile = values[Math.ceil(values.length * 0.75) - 1];
    recommendations[language.Name] = Math.max(1, percentile);
  }
  return recommendations;
}

export async function runSkylos(cwd, files, configPath) {
  if (!files.length) return {};
  const args = [
    '--config-file', configPath,
    '--danger', '--secrets', '--quality',
    '--format', 'json', '--no-provenance',
    ...files
  ];
  const { stdout } = await run('skylos', args, { cwd });
  return parseJson(stdout, 'Skylos');
}

export function newSkylosFindings(current, baseline, currentRoot, baseRoot, allowedPaths) {
  const allowed = new Set(allowedPaths.map(normalizePath));
  const before = collectFindings(baseline, baseRoot, allowed);
  const fingerprints = new Set(before.map(fingerprint));
  return collectFindings(current, currentRoot, allowed).filter((finding) => !fingerprints.has(fingerprint(finding)));
}

export async function runNativeSkylosGate(findings) {
  const tempRoot = await mkdtemp(path.join(os.tmpdir(), 'sonata-skylos-gate-'));
  const reportPath = path.join(tempRoot, 'findings.json');
  const filtered = {};
  for (const category of SKYLOS_CATEGORIES) filtered[category] = [];
  for (const finding of findings) {
    if (!Array.isArray(filtered[finding.category])) filtered[finding.category] = [];
    filtered[finding.category].push(finding.raw);
  }
  await writeFile(reportPath, `${JSON.stringify(filtered)}\n`);
  try {
    await run('skylos', ['cicd', 'gate', '--input', reportPath, '--strict']);
  } catch (error) {
    if (!findings.length) throw new Error(`Skylos native gate failed without changed-code findings: ${toolError(error)}`);
  } finally {
    await rm(tempRoot, { recursive: true, force: true });
  }
}

export function formatSkylosFindings(findings) {
  return findings.map(({ path: file, raw }) =>
    `${file}:${raw.line || 1}: ${raw.rule_id || 'Skylos'} ${raw.message || 'policy violation'}`
  );
}

export async function cleanup(directory) {
  if (directory) await rm(directory, { recursive: true, force: true });
}

function collectFindings(report, root, allowed) {
  const findings = [];
  for (const category of SKYLOS_CATEGORIES) {
    for (const raw of report?.[category] || []) {
      const file = normalizeFindingPath(raw.file, root);
      if (file && allowed.has(file)) findings.push({ category, path: file, raw });
    }
  }
  return findings;
}

function fingerprint({ category, path: file, raw }) {
  return [category, file, raw.rule_id, raw.severity, raw.name || raw.simple_name, raw.message].join('\0');
}

function normalizeFindingPath(file, root) {
  if (!file) return null;
  const relative = path.isAbsolute(file) ? path.relative(root, file) : file;
  if (relative.startsWith('..')) return null;
  return normalizePath(relative);
}

function normalizePath(file) {
  return file.split(path.sep).join('/').replace(/^\.\//, '');
}

function safeTarget(root, relative) {
  const target = path.resolve(root, relative);
  if (target !== root && !target.startsWith(`${root}${path.sep}`)) throw new Error(`Unsafe Git path: ${relative}`);
  return target;
}

function parseJson(output, label) {
  const start = Math.min(...['{', '['].map((token) => {
    const index = output.indexOf(token);
    return index === -1 ? Number.POSITIVE_INFINITY : index;
  }));
  if (!Number.isFinite(start)) throw new Error(`${label} did not return JSON.`);
  try {
    return JSON.parse(output.slice(start));
  } catch (error) {
    throw new Error(`${label} returned invalid JSON: ${error.message}`);
  }
}

async function git(cwd, args) {
  return (await run('git', args, { cwd })).stdout;
}

async function gitBuffer(cwd, args) {
  const result = await execFileAsync('git', args, { cwd, encoding: 'buffer', maxBuffer: MAX_BUFFER });
  return result.stdout;
}

async function run(command, args, options = {}) {
  try {
    return await execFileAsync(command, args, { ...options, encoding: 'utf8', maxBuffer: MAX_BUFFER });
  } catch (error) {
    error.message = `${command} failed: ${toolError(error)}`;
    throw error;
  }
}

function toolError(error) {
  return String(error.stderr || error.stdout || error.message).trim();
}
