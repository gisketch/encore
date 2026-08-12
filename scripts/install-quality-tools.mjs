#!/usr/bin/env node
import { execFile } from 'node:child_process';
import { appendFile, chmod, mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import path from 'node:path';
import process from 'node:process';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const root = process.cwd();

try {
  const policy = JSON.parse(await readFile(path.join(root, '.sonata', 'quality-gates.json'), 'utf8'));
  for (const tool of ['scc', 'skylos']) {
    if (policy[tool]?.enabled && !/^\d+\.\d+\.\d+$/.test(policy[tool].version || '')) {
      throw new Error(`${tool} requires an exact x.y.z version in .sonata/quality-gates.json.`);
    }
  }
  if (policy.scc?.enabled) await installScc(policy.scc.version);
  if (policy.skylos?.enabled) await installSkylos(policy.skylos.version);
  if (!policy.scc?.enabled && !policy.skylos?.enabled) console.log('quality tools disabled');
} catch (error) {
  console.error(`quality tool install failed: ${error.message}`);
  process.exitCode = 1;
}

async function installScc(version) {
  if (process.platform === 'win32') {
    throw new Error('Automatic SCC installation currently supports Linux and macOS CI. Install the pinned Windows release manually.');
  }
  const platform = process.platform === 'darwin' ? 'Darwin' : process.platform === 'linux' ? 'Linux' : null;
  const architecture = process.arch === 'arm64' ? 'arm64' : process.arch === 'x64' ? 'x86_64' : null;
  if (!platform || !architecture) throw new Error(`Unsupported SCC platform: ${process.platform}/${process.arch}`);

  const name = `scc_${platform}_${architecture}.tar.gz`;
  const release = `https://github.com/boyter/scc/releases/download/v${version}`;
  const [archive, checksums] = await Promise.all([
    download(`${release}/${name}`),
    download(`${release}/checksums.txt`, true)
  ]);
  const expected = checksums.split('\n').find((line) => line.trim().endsWith(name))?.trim().split(/\s+/)[0];
  const actual = createHash('sha256').update(archive).digest('hex');
  if (!expected || actual !== expected) throw new Error(`SCC ${version} checksum verification failed.`);

  const binDir = path.join(root, '.sonata', 'bin');
  const archivePath = path.join(binDir, name);
  await mkdir(binDir, { recursive: true });
  await writeFile(archivePath, archive);
  await execFileAsync('tar', ['-xzf', archivePath, '-C', binDir]);
  await chmod(path.join(binDir, 'scc'), 0o755);
  if (process.env.GITHUB_PATH) await appendFile(process.env.GITHUB_PATH, `${binDir}\n`);
  console.log(`installed SCC ${version} in ${path.relative(root, binDir)}`);
}

async function installSkylos(version) {
  const python = await findPython();
  await execFileAsync(python, ['-m', 'pip', 'install', `skylos==${version}`], { maxBuffer: 20 * 1024 * 1024 });
  console.log(`installed Skylos ${version}`);
}

async function findPython() {
  for (const command of ['python3', 'python']) {
    try {
      await execFileAsync(command, ['--version']);
      return command;
    } catch {}
  }
  throw new Error('Python is required to install Skylos.');
}

async function download(url, asText = false) {
  const response = await fetch(url, { redirect: 'follow' });
  if (!response.ok) throw new Error(`Download failed (${response.status}): ${url}`);
  return asText ? response.text() : Buffer.from(await response.arrayBuffer());
}
