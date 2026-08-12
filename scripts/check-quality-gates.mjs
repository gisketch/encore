#!/usr/bin/env node
import path from 'node:path';
import process from 'node:process';
import {
  changedFiles,
  cleanup,
  evaluateScc,
  formatSkylosFindings,
  materializeBase,
  newSkylosFindings,
  readPolicy,
  recommendScc,
  resolveBase,
  runNativeSkylosGate,
  runScc,
  runSkylos,
  verifyTool
} from './quality-gates.mjs';

const root = process.cwd();
process.env.PATH = `${path.join(root, '.sonata', 'bin')}${path.delimiter}${process.env.PATH || ''}`;

try {
  const options = parseArgs(process.argv.slice(2));
  const policy = await readPolicy(root);
  if (options.recommendScc) {
    await verifyTool('scc', policy.scc.version);
    console.log(JSON.stringify(await recommendScc(root), null, 2));
  } else {
    await runGate(policy, options);
  }
} catch (error) {
  console.error(`quality gate failed: ${error.message}`);
  process.exitCode = 1;
}

async function runGate(policy, options) {
  const enabled = [policy.scc?.enabled && 'SCC', policy.skylos?.enabled && 'Skylos'].filter(Boolean);
  if (!enabled.length) {
    console.log('quality gates disabled');
    return;
  }
  if (policy.scc?.enabled) await verifyTool('scc', policy.scc.version);
  if (policy.skylos?.enabled) await verifyTool('skylos', policy.skylos.version);

  const base = await resolveBase(root, options.base);
  const changes = await changedFiles(root, base);
  const files = changes.map(({ path: file }) => file);
  if (!files.length) {
    console.log(`quality gates ok (${enabled.join(', ')}; no changed files)`);
    return;
  }

  let baseRoot;
  const failures = [];
  try {
    baseRoot = await materializeBase(root, base, changes);
    if (policy.scc?.enabled) {
      const current = await runScc(root, files);
      const baseline = await runScc(baseRoot, changes.filter(({ status }) => status !== 'A').map(({ path: file }) => file));
      failures.push(...evaluateScc(current, baseline, changes, policy.scc.thresholds));
    }
    if (policy.skylos?.enabled) {
      const config = path.resolve(root, policy.skylos.config || '.sonata/skylos.toml');
      const current = await runSkylos(root, files, config);
      const baselineFiles = changes.filter(({ status }) => status !== 'A').map(({ path: file }) => file);
      const baseline = await runSkylos(baseRoot, baselineFiles, config);
      const findings = newSkylosFindings(current, baseline, root, baseRoot, files);
      await runNativeSkylosGate(findings);
      failures.push(...formatSkylosFindings(findings));
    }
  } finally {
    await cleanup(baseRoot);
  }

  if (failures.length) throw new Error(`changed code violates policy:\n- ${failures.join('\n- ')}`);
  console.log(`quality gates ok (${enabled.join(', ')}; base ${base})`);
}

function parseArgs(args) {
  const options = {};
  for (let index = 0; index < args.length; index += 1) {
    if (args[index] === '--base') {
      if (!args[index + 1]) throw new Error('--base requires a Git ref.');
      options.base = args[++index];
    } else if (args[index] === '--recommend-scc') {
      options.recommendScc = true;
    } else if (args[index] === '--help' || args[index] === '-h') {
      console.log('Usage: node scripts/check-quality-gates.mjs [--base <ref>] [--recommend-scc]');
      process.exit(0);
    } else {
      throw new Error(`Unknown option: ${args[index]}`);
    }
  }
  return options;
}
