// Runs bump-version-up.sh against fixture copies in a temp dir (never the real repo files).
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const SCRIPT = join(dirname(fileURLToPath(import.meta.url)), "..", "bump-version-up.sh");

const manifest = (name) =>
  `[package]\nname = "${name}"\nversion = "0.3.1"\nedition = "2024"\n\n[dependencies]\nserde = { version = "1" }\n`;

const LOCK = [
  "version = 4",
  ...["kiri-app", "kiri-core", "kiri-cli", "unrelated"].map((n) => `[[package]]\nname = "${n}"\nversion = "0.3.1"\n`),
].join("\n\n");

const FIXTURES = {
  "package.json": `{\n  "name": "kiri",\n  "version": "0.3.1",\n  "devDependencies": {\n    "x": "1.0.0"\n  }\n}\n`,
  "src-tauri/tauri.conf.json": `{\n  "productName": "kiri",\n  "version": "0.3.1"\n}\n`,
  "src-tauri/Cargo.toml": manifest("kiri-app"),
  "crates/kiri-core/Cargo.toml": manifest("kiri-core"),
  "crates/kiri-cli/Cargo.toml": manifest("kiri-cli"),
  "Cargo.lock": LOCK,
};

function setup(overrides = {}) {
  const dir = mkdtempSync(join(tmpdir(), "kiri-bump-"));
  copyFileSync(SCRIPT, join(dir, "bump-version-up.sh"));
  for (const [path, body] of Object.entries({ ...FIXTURES, ...overrides })) {
    mkdirSync(dirname(join(dir, path)), { recursive: true });
    writeFileSync(join(dir, path), body);
  }
  return dir;
}

const read = (dir, path) => readFileSync(join(dir, path), "utf8");
const run = (dir, type) => execFileSync("bash", [join(dir, "bump-version-up.sh"), type], { stdio: "pipe" });

function lockVersion(lock, name) {
  return lock.match(new RegExp(`name = "${name}"\\nversion = "([^"]+)"`))[1];
}

void test("patch bumps every file and every workspace crate", () => {
  const dir = setup();
  run(dir, "patch");
  assert.match(read(dir, "package.json"), /"version": "0\.3\.2"/);
  assert.match(read(dir, "package.json"), /"x": "1\.0\.0"/);
  assert.match(read(dir, "src-tauri/tauri.conf.json"), /"version": "0\.3\.2"/);
  for (const path of ["src-tauri/Cargo.toml", "crates/kiri-core/Cargo.toml", "crates/kiri-cli/Cargo.toml"]) {
    assert.match(read(dir, path), /^version = "0\.3\.2"$/m, path);
    assert.match(read(dir, path), /serde = \{ version = "1" \}/, path);
  }
  const lock = read(dir, "Cargo.lock");
  for (const name of ["kiri-app", "kiri-core", "kiri-cli"]) {
    assert.equal(lockVersion(lock, name), "0.3.2", name);
  }
  assert.equal(lockVersion(lock, "unrelated"), "0.3.1");
});

void test("minor and major reset the lower parts", () => {
  const dir = setup();
  run(dir, "minor");
  assert.match(read(dir, "src-tauri/tauri.conf.json"), /"version": "0\.4\.0"/);
  run(dir, "major");
  assert.match(read(dir, "src-tauri/tauri.conf.json"), /"version": "1\.0\.0"/);
  assert.equal(lockVersion(read(dir, "Cargo.lock"), "kiri-cli"), "1.0.0");
});

void test("a crate missing from Cargo.lock fails and restores every file", () => {
  const lock = LOCK.replace('name = "kiri-cli"', 'name = "other"');
  const dir = setup({ "Cargo.lock": lock });
  assert.throws(() => run(dir, "patch"));
  for (const [path, body] of Object.entries({ ...FIXTURES, "Cargo.lock": lock })) {
    assert.equal(read(dir, path), body, path);
  }
});
