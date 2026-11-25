#!/usr/bin/env node
// Release helpers, used by CI and runnable by hand from the repository root:
//
//   node scripts/release.mjs check-versions
//   node scripts/release.mjs prepare --dir <downloaded build artifacts> --version 1.0.6 --repo owner/repo
//
// `prepare` renames the installers, adds version-less copies for stable download links, and writes
// the updater manifest. It fails when any file a platform needs is missing, so a broken release is
// caught on the pull request instead of after it is published.

import { copyFileSync, mkdirSync, readFileSync, readdirSync, renameSync, statSync, writeFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const ROOT = fileURLToPath(new URL("../", import.meta.url));

/** Every file a release must carry. A bundle file matches the first pattern it ends with. */
const RELEASE_FILES = [
  { id: "mac-dmg", match: /\.dmg$/, label: "macOS disk image", stable: "Wecun-universal.dmg" },
  { id: "mac-updater", match: /\.app\.tar\.gz$/, label: "macOS updater bundle" },
  { id: "mac-updater-sig", match: /\.app\.tar\.gz\.sig$/, label: "macOS updater signature" },
  { id: "windows-installer", match: /-setup\.exe$/, label: "Windows installer", stable: "Wecun-x64-setup.exe" },
  { id: "windows-updater", match: /\.nsis\.zip$/, label: "Windows updater bundle" },
  { id: "windows-updater-sig", match: /\.nsis\.zip\.sig$/, label: "Windows updater signature" },
  { id: "linux-appimage", match: /\.AppImage$/, label: "Linux AppImage", stable: "Wecun-latest.AppImage" },
  { id: "linux-appimage-updater", match: /\.AppImage\.tar\.gz$/, label: "Linux AppImage updater bundle" },
  { id: "linux-appimage-updater-sig", match: /\.AppImage\.tar\.gz\.sig$/, label: "Linux AppImage updater signature" },
  { id: "linux-deb", match: /\.deb$/, label: "Linux package", stable: "Wecun-latest.deb" },
  { id: "linux-deb-sig", match: /\.deb\.sig$/, label: "Linux package updater signature" },
];

/** The updater looks a platform up by `{os}-{arch}-{installer}`, so every installer gets its own key. */
const PLATFORMS = {
  "darwin-universal": ["mac-updater", "mac-updater-sig"],
  "darwin-aarch64": ["mac-updater", "mac-updater-sig"],
  "darwin-x86_64": ["mac-updater", "mac-updater-sig"],
  "windows-x86_64-nsis": ["windows-updater", "windows-updater-sig"],
  "linux-x86_64-appimage": ["linux-appimage-updater", "linux-appimage-updater-sig"],
  "linux-x86_64-deb": ["linux-deb", "linux-deb-sig"],
};

/** Every place the app version is written. If they disagree, builds and releases disagree. */
export function findVersionMismatches(files) {
  const expected = files.packageJson.version;
  const versions = {
    "package.json": expected,
    "package-lock.json": files.packageLock.version,
    'package-lock.json "packages"': files.packageLock.packages?.[""]?.version,
    "src-tauri/tauri.conf.json": files.tauriConf.version,
    "src-tauri/Cargo.toml": /^version\s*=\s*"([^"]+)"/m.exec(files.cargoToml)?.[1],
  };
  return Object.entries(versions)
    .filter(([, version]) => version !== expected)
    .map(([file, version]) => `${file} says ${version ?? "<missing>"}, package.json says ${expected}`);
}

/** Bundle files are named after the product ("微存_…", "微存.app…"); releases use "Wecun". */
export function releaseName(fileName) {
  return "Wecun" + fileName.replace(/^(微存|Wecun|wecun)(?=[_.])/, "");
}

function checkVersions() {
  const read = (path) => readFileSync(join(ROOT, path), "utf8");
  const files = {
    packageJson: JSON.parse(read("package.json")),
    packageLock: JSON.parse(read("package-lock.json")),
    tauriConf: JSON.parse(read("src-tauri/tauri.conf.json")),
    cargoToml: read("src-tauri/Cargo.toml"),
  };
  const mismatches = findVersionMismatches(files);
  if (mismatches.length > 0) {
    console.error(`Version numbers disagree:\n  ${mismatches.join("\n  ")}`);
    process.exitCode = 1;
    return;
  }
  console.log(`Version ${files.packageJson.version} agrees everywhere.`);
}

function listFiles(dir, skip = () => false) {
  return readdirSync(dir).flatMap((entry) => {
    const path = join(dir, entry);
    if (skip(path)) return [];
    return statSync(path).isDirectory() ? listFiles(path, skip) : [path];
  });
}

function prepare({ dir, version, repo }) {
  const outDir = join(dir, "release");
  const found = new Map();
  const seen = [];
  const problems = [];
  for (const path of listFiles(dir, (path) => path.startsWith(outDir))) {
    const spec = RELEASE_FILES.find((spec) => spec.match.test(basename(path)));
    seen.push(basename(path));
    if (!spec) continue;
    if (found.has(spec)) {
      problems.push(`two files look like the ${spec.label}: ${basename(found.get(spec))} and ${basename(path)}`);
    } else {
      found.set(spec, path);
    }
  }
  for (const spec of RELEASE_FILES) {
    if (!found.has(spec)) problems.push(`missing the ${spec.label}`);
  }
  if (problems.length > 0) {
    console.error(
      `Cannot assemble v${version}:\n  ${problems.join("\n  ")}\nFiles that were found:\n  ${seen.join("\n  ") || "<none>"}`,
    );
    process.exitCode = 1;
    return;
  }

  mkdirSync(outDir, { recursive: true });
  const names = new Map();
  for (const [spec, path] of found) {
    const name = releaseName(basename(path));
    renameSync(path, join(outDir, name));
    names.set(spec.id, name);
    if (spec.stable) copyFileSync(join(outDir, name), join(outDir, spec.stable));
  }

  const url = (id) => `https://github.com/${repo}/releases/download/v${version}/${names.get(id)}`;
  const signature = (id) => readFileSync(join(outDir, names.get(id)), "utf8").trim();
  const manifest = {
    version: `v${version}`,
    notes: `See the release notes at https://github.com/${repo}/releases/tag/v${version}`,
    pub_date: new Date().toISOString(),
    platforms: Object.fromEntries(
      Object.entries(PLATFORMS).map(([platform, [bundle, sig]]) => [
        platform,
        { signature: signature(sig), url: url(bundle) },
      ]),
    ),
  };
  writeFileSync(join(outDir, "latest.json"), `${JSON.stringify(manifest, null, 2)}\n`);

  console.log(`Assembled v${version} in ${outDir}:`);
  for (const name of names.values()) console.log(`  ${name}`);
  console.log(`Updater platforms: ${Object.keys(manifest.platforms).join(", ")}`);
}

function main() {
  const [command] = process.argv.slice(2);
  if (command === "check-versions") {
    checkVersions();
    return;
  }
  if (command === "prepare") {
    const { values } = parseArgs({
      args: process.argv.slice(3),
      options: { dir: { type: "string" }, version: { type: "string" }, repo: { type: "string" } },
    });
    if (!values.dir || !values.version || !values.repo) {
      console.error("usage: node scripts/release.mjs prepare --dir <folder> --version <x.y.z> --repo <owner/repo>");
      process.exitCode = 1;
      return;
    }
    prepare(values);
    return;
  }
  console.error("usage: node scripts/release.mjs <check-versions|prepare>");
  process.exitCode = 1;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main();
}
