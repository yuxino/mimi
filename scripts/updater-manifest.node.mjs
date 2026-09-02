import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  createUpdaterManifest,
  expectedUpdaterAssets,
  verifyUpdaterManifest,
} from "./updater-manifest.mjs";

const version = "2.3.4";
const repository = "yuxino/mimi";
const tag = `v${version}`;
const pubDate = "2026-09-02T05:00:00Z";

function signature(label) {
  return Buffer.from(
    `untrusted comment: signature from tauri secret key\n${label}\ntrusted comment: timestamp:1\n${label}`,
  ).toString("base64");
}

function fixture() {
  const assetDir = mkdtempSync(join(tmpdir(), "mimi-updater-manifest-"));
  const assets = expectedUpdaterAssets(version);
  for (const [key, name] of Object.entries(assets)) {
    writeFileSync(
      join(assetDir, name),
      key.endsWith("Signature") ? signature(key) : `fixture:${name}`,
    );
  }
  return { assetDir, assets };
}

test("creates a two-platform manifest bound to signed release assets", () => {
  const { assetDir, assets } = fixture();
  try {
    const manifest = createUpdaterManifest({
      assetDir,
      version,
      repository,
      tag,
      pubDate,
      notes: "Signed update fixture",
    });
    assert.equal(manifest.version, version);
    assert.deepEqual(Object.keys(manifest.platforms).sort(), [
      "darwin-aarch64",
      "windows-x86_64",
    ]);
    assert.match(manifest.platforms["darwin-aarch64"].url, /mimi\.app\.tar\.gz$/);
    assert.match(manifest.platforms["windows-x86_64"].url, /x64-setup\.exe$/);
    assert.equal(
      manifest.platforms["darwin-aarch64"].signature,
      readFileSync(join(assetDir, assets.macSignature), "utf8"),
    );
  } finally {
    rmSync(assetDir, { recursive: true, force: true });
  }
});

test("rejects a missing or empty signature", () => {
  const { assetDir, assets } = fixture();
  try {
    writeFileSync(join(assetDir, assets.windowsExeSignature), "");
    assert.throws(
      () =>
        createUpdaterManifest({
          assetDir,
          version,
          repository,
          tag,
          pubDate,
          notes: "fixture",
        }),
      /empty release asset/,
    );
  } finally {
    rmSync(assetDir, { recursive: true, force: true });
  }
});

test("rejects a manifest URL that is not bound to the release tag", () => {
  const { assetDir } = fixture();
  try {
    const manifest = createUpdaterManifest({
      assetDir,
      version,
      repository,
      tag,
      pubDate,
      notes: "fixture",
    });
    manifest.platforms["darwin-aarch64"].url =
      "https://example.com/mimi.app.tar.gz";
    assert.throws(
      () =>
        verifyUpdaterManifest({
          manifest,
          assetDir,
          version,
          repository,
          tag,
        }),
      /does not match its release asset/,
    );
  } finally {
    rmSync(assetDir, { recursive: true, force: true });
  }
});
