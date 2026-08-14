#!/usr/bin/env node
// Regenerates the post-save confirmation chime shipped as a bundle resource.
//
//   node scripts/generate-save-chime.mjs
//
// A soft two-tone chime (A5 -> E6) with a fast attack and an exponential
// decay, 44.1 kHz, mono, 16-bit PCM, ~0.3 s (~26 KB). Deliberately
// dependency-free and deterministic: rerunning it reproduces the committed
// `src-tauri/resources/save-chime.wav` byte for byte.
import { writeFileSync } from 'node:fs';
import path from 'node:path';

const RATE = 44100;
const SECONDS = 0.3;
const OUTPUT = path.join('src-tauri', 'resources', 'save-chime.wav');
// Start second, decay rate, peak amplitude, frequency (Hz).
const TONES = [
  { start: 0.0, decay: 13, gain: 0.5, hz: 880 },
  { start: 0.09, decay: 11, gain: 0.42, hz: 1320 }
];

const frames = Math.round(RATE * SECONDS);
const samples = Buffer.alloc(frames * 2);
for (let frame = 0; frame < frames; frame += 1) {
  const time = frame / RATE;
  const value = TONES.reduce((sum, tone) => sum + voice(tone, time), 0);
  const eased = value * fade(frame / frames);
  samples.writeInt16LE(Math.round(Math.max(-1, Math.min(1, eased)) * 32767), frame * 2);
}
writeFileSync(OUTPUT, Buffer.concat([header(samples.length), samples]));
console.log(`${OUTPUT}: ${44 + samples.length} bytes`);

// One decaying sine partial; silent before its start, so the second tone
// arrives while the first is still ringing.
function voice({ start, decay, gain, hz }, time) {
  const age = time - start;
  const audible = age >= 0 ? 1 : 0;
  return audible * gain * Math.exp(-decay * age) * Math.sin(2 * Math.PI * hz * age);
}

// A short raised-cosine attack plus a tail fade, so neither end clicks.
function fade(progress) {
  const attack = Math.min(1, progress / 0.02);
  const release = Math.min(1, (1 - progress) / 0.08);
  return 0.5 * (1 - Math.cos(Math.PI * attack)) * release;
}

function header(dataBytes) {
  const buffer = Buffer.alloc(44);
  buffer.write('RIFF', 0);
  buffer.writeUInt32LE(36 + dataBytes, 4);
  buffer.write('WAVEfmt ', 8);
  buffer.writeUInt32LE(16, 16); // PCM chunk size
  buffer.writeUInt16LE(1, 20); // PCM
  buffer.writeUInt16LE(1, 22); // mono
  buffer.writeUInt32LE(RATE, 24);
  buffer.writeUInt32LE(RATE * 2, 28); // byte rate
  buffer.writeUInt16LE(2, 32); // block align
  buffer.writeUInt16LE(16, 34); // bits per sample
  buffer.write('data', 36);
  buffer.writeUInt32LE(dataBytes, 40);
  return buffer;
}
