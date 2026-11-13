"use strict";

const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

// Asset names, sizes and upload order from
// GET /repos/arturkmat/rustdesk/releases/tags/dark-smart on 2026-11-13.
// GitHub lists 1.5.0 before 1.5.1; rustdesk.sbom.json is not an installer.
const release = {
  name: "dark-smart",
  tag_name: "dark-smart",
  assets: [
    ["rustdesk-1.5.0-aarch64.apk", 27245751],
    ["rustdesk-1.5.0-aarch64.exe", 22658560],
    ["rustdesk-1.5.0-aarch64.msi", 23098219],
    ["rustdesk-1.5.0-armv7.apk", 25832731],
    ["rustdesk-1.5.0-universal.apk", 72183737],
    ["rustdesk-1.5.0-x86-sciter.exe", 11995648],
    ["rustdesk-1.5.0-x86_64.apk", 26631964],
    ["rustdesk-1.5.0-x86_64.exe", 24791552],
    ["rustdesk-1.5.0-x86_64.msi", 25096960],
    ["rustdesk-1.5.1-aarch64.apk", 29648203],
    ["rustdesk-1.5.1-aarch64.exe", 24516608],
    ["rustdesk-1.5.1-aarch64.msi", 25039873],
    ["rustdesk-1.5.1-armv7.apk", 28058839],
    ["rustdesk-1.5.1-universal.apk", 79317808],
    ["rustdesk-1.5.1-x86_64.apk", 29165925],
    ["rustdesk-1.5.1-x86_64.exe", 26479104],
    ["rustdesk-1.5.1-x86_64.msi", 26895254],
    ["rustdesk.sbom.json", 1868517],
  ].map(([name, size]) => ({
    name,
    size,
    created_at: name.includes("1.5.1") ? "2026-11-13T09:06:04Z" : "2026-09-23T00:03:14Z",
    browser_download_url: `https://github.com/arturkmat/rustdesk/releases/download/dark-smart/${name}`,
    content_type: "application/octet-stream",
  })),
};

function makeEl() {
  const el = {
    className: "",
    href: "",
    textContent: "",
    children: [],
    classList: {
      add() {},
      remove() {},
    },
    appendChild(child) {
      this.children.push(child);
    },
  };
  let html = "";
  Object.defineProperty(el, "innerHTML", {
    get() {
      return html;
    },
    set(value) {
      html = String(value);
      el.children = [];
    },
  });
  return el;
}

function pageContext(userAgent, platform) {
  const files = {
    windows: makeEl(),
    macos: makeEl(),
    linux: makeEl(),
    android: makeEl(),
  };
  const byId = {
    status: makeEl(),
    primary: makeEl(),
    detected: makeEl(),
  };
  const html = fs.readFileSync(path.join(__dirname, "index.html"), "utf8");
  const script = html.match(/<script>([\s\S]*)<\/script>/)[1].replace(/\bload\(\);\s*$/, "");
  const context = {
    document: {
      querySelector(sel) {
        const match = sel.match(/data-os="(\w+)"/);
        return files[match[1]];
      },
      getElementById(id) {
        return byId[id];
      },
      createElement() {
        return makeEl();
      },
    },
    navigator: { userAgent, platform },
    fetch: async (url) => {
      assert.match(String(url), /\/releases\/tags\/dark-smart$/);
      return { ok: true, json: async () => release };
    },
    console,
  };
  vm.createContext(context);
  vm.runInContext(script, context);
  return { context, files, byId };
}

function hrefs(el) {
  return el.children.map((child) => child.href);
}

async function main() {
  const win = pageContext(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
    "Win32"
  );
  await win.context.load();

  assert.equal(
    win.byId.primary.href,
    "https://github.com/arturkmat/rustdesk/releases/download/dark-smart/rustdesk-1.5.1-x86_64.msi"
  );
  assert.equal(win.byId.primary.textContent, "Pobierz wersję 1.5.1 dla Windows");
  assert.equal(win.byId.status.textContent, "Najnowsza wersja: 1.5.1");
  assert.equal(win.byId.detected.textContent, "Wykryty system: Windows");

  const windowsHrefs = hrefs(win.files.windows);
  assert.deepEqual(windowsHrefs, [
    "https://github.com/arturkmat/rustdesk/releases/download/dark-smart/rustdesk-1.5.1-x86_64.msi",
    "https://github.com/arturkmat/rustdesk/releases/download/dark-smart/rustdesk-1.5.1-x86_64.exe",
    "https://github.com/arturkmat/rustdesk/releases/download/dark-smart/rustdesk-1.5.1-aarch64.msi",
    "https://github.com/arturkmat/rustdesk/releases/download/dark-smart/rustdesk-1.5.1-aarch64.exe",
    "https://github.com/arturkmat/rustdesk/releases/download/dark-smart/rustdesk-1.5.0-x86-sciter.exe",
  ]);
  assert.match(win.files.windows.children[0].innerHTML, /<b>MSI · 64-bit<\/b><span>1\.5\.1 · /);
  assert.match(win.files.windows.children[4].innerHTML, /<span>1\.5\.0 · /);
  assert.equal(
    windowsHrefs.some((href) => href.endsWith("rustdesk-1.5.0-x86_64.msi")),
    false
  );

  const allHrefs = Object.values(win.files).flatMap(hrefs);
  assert.equal(allHrefs.some((href) => href.endsWith("rustdesk.sbom.json")), false);
  assert.match(win.files.macos.innerHTML, /Jeszcze nie ma paczki/);
  assert.match(win.files.linux.innerHTML, /Jeszcze nie ma paczki/);

  const android = pageContext(
    "Mozilla/5.0 (Linux; Android 14; Pixel 8)",
    "Linux armv8l"
  );
  await android.context.load();
  assert.equal(
    android.byId.primary.href,
    "https://github.com/arturkmat/rustdesk/releases/download/dark-smart/rustdesk-1.5.1-aarch64.apk"
  );
  assert.equal(android.byId.primary.textContent, "Pobierz wersję 1.5.1 dla Android");
  const androidHrefs = hrefs(android.files.android);
  assert.equal(androidHrefs.some((href) => href.includes("1.5.0")), false);
  assert.match(android.files.android.children[0].innerHTML, /<b>APK · ARM64<\/b><span>1\.5\.1 · /);

  const numeric = pageContext(
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
    "Win32"
  );
  numeric.context.fetch = async () => ({
    ok: true,
    json: async () => ({
      name: "dark-smart",
      tag_name: "dark-smart",
      assets: [
        {
          name: "rustdesk-1.5.9-x86_64.msi",
          size: 10,
          browser_download_url: "https://example.invalid/1.5.9.msi",
        },
        {
          name: "rustdesk-1.5.10-x86_64.exe",
          size: 10,
          browser_download_url: "https://example.invalid/1.5.10.exe",
        },
        {
          name: "rustdesk-1.5.10-x86_64.msi",
          size: 11,
          browser_download_url: "https://example.invalid/1.5.10.msi",
        },
      ],
    }),
  });
  await numeric.context.load();
  assert.equal(numeric.byId.primary.href, "https://example.invalid/1.5.10.msi");
  assert.deepEqual(hrefs(numeric.files.windows), [
    "https://example.invalid/1.5.10.msi",
    "https://example.invalid/1.5.10.exe",
  ]);
  assert.equal(numeric.byId.status.textContent, "Najnowsza wersja: 1.5.10");

  console.log("pages/download.test.js: ok");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
