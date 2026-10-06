#!/usr/bin/env node
// 릴리스에 올라온 .app.tar.gz.sig 로 latest.json 을 만들어 같은 릴리스에 올린다.
// 사용: node scripts/latest-json.mjs v0.2.0
import { execFileSync } from "node:child_process";
import { mkdtempSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const REPO = "bob-park/kiri-app";
const PLATFORMS = [{ suffix: ".app.tar.gz.sig", key: "darwin-aarch64" }];

/**
 * @param {string} tag
 * @param {{name: string, body: string}[]} sigs
 * @param {Date} [now]
 */
export function buildManifest(tag, sigs, now = new Date()) {
  /** @type {Record<string, {signature: string, url: string}>} */
  const platforms = {};
  for (const { suffix, key } of PLATFORMS) {
    const sig = sigs.find((s) => s.name.endsWith(suffix));
    if (!sig) continue;
    const asset = sig.name.slice(0, -".sig".length);
    platforms[key] = {
      signature: sig.body.trim(),
      url: `https://github.com/${REPO}/releases/download/${tag}/${asset}`,
    };
  }
  return { version: tag.replace(/^v/, ""), pub_date: now.toISOString(), platforms };
}

const isMain = process.argv[1] && fileURLToPath(import.meta.url) === resolve(process.argv[1]);
if (isMain) {
  const tag = process.argv[2];
  if (!tag) {
    console.error("usage: node scripts/latest-json.mjs <tag>");
    process.exit(1);
  }
  // 공개키가 비어 있으면 그 빌드는 업데이트를 검증하지 못한다. 받은 뒤에야 실패하므로 여기서 막는다.
  const conf = JSON.parse(readFileSync(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"));
  if (!conf.plugins?.updater?.pubkey || conf.plugins.updater.pubkey === "<PUBKEY>") {
    console.error("src-tauri/tauri.conf.json: plugins.updater.pubkey is empty (see docs/development.md)");
    process.exit(1);
  }
  const dir = mkdtempSync(join(tmpdir(), "kiri-sig-"));
  execFileSync("gh", ["release", "download", tag, "-R", REPO, "-p", "*.sig", "-D", dir], { stdio: "inherit" });
  const sigs = readdirSync(dir).map((name) => ({ name, body: readFileSync(join(dir, name), "utf8") }));
  const manifest = buildManifest(tag, sigs);
  if (Object.keys(manifest.platforms).length === 0) {
    console.error(`no .app.tar.gz.sig on release ${tag}`);
    process.exit(1);
  }
  const out = join(dir, "latest.json");
  writeFileSync(out, JSON.stringify(manifest, null, 2) + "\n");
  execFileSync("gh", ["release", "upload", tag, "-R", REPO, out, "--clobber"], { stdio: "inherit" });
  console.log(`latest.json uploaded to ${tag}`);
}
