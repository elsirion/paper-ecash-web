export async function createWalletWorker() {
  const url = new URL("wallet-worker.js", self.location.href);
  const worker = new Worker(url, {
    type: "module",
    name: "paper-ecash-wallet"
  });
  await new Promise((resolve) => {
    const handler = (event) => {
      if (event.data === "__ready__") {
        worker.removeEventListener("message", handler);
        resolve();
      }
    };
    worker.addEventListener("message", handler);
  });
  return worker;
}

export function supportsSyncAccessHandles() {
  return (
    typeof FileSystemFileHandle !== "undefined" &&
    typeof FileSystemFileHandle.prototype?.createSyncAccessHandle === "function"
  );
}

export async function openWalletDb(fileName) {
  const root = await navigator.storage.getDirectory();
  const handle = await root.getFileHandle(fileName, { create: true });
  if (typeof handle.createSyncAccessHandle !== "function") {
    throw new Error(
      "This browser does not support OPFS Sync Access Handles. Use a recent Chromium-based browser."
    );
  }
  return await handle.createSyncAccessHandle();
}

export async function copyText(value) {
  if (navigator.clipboard) {
    await navigator.clipboard.writeText(value);
  }
}

export function downloadBlob(bytes, filename, mimeType) {
  const blob = new Blob([bytes], { type: mimeType });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  setTimeout(() => {
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  }, 100);
}

export async function fetchFontWoff2(family) {
  const cssUrl = `https://fonts.googleapis.com/css2?family=${encodeURIComponent(family)}`;
  const cssResp = await fetch(cssUrl);
  if (!cssResp.ok) {
    throw new Error(`Failed to fetch font CSS for "${family}": ${cssResp.status}`);
  }
  const css = await cssResp.text();
  const match = css.match(/url\((https:\/\/fonts\.gstatic\.com\/[^)]+\.woff2)\)/);
  if (!match) {
    throw new Error(`No woff2 URL found in CSS for "${family}"`);
  }
  const woff2Url = match[1];
  const fontResp = await fetch(woff2Url);
  if (!fontResp.ok) {
    throw new Error(`Failed to fetch woff2 for "${family}": ${fontResp.status}`);
  }
  const buf = await fontResp.arrayBuffer();
  return { url: woff2Url, bytes: new Uint8Array(buf) };
}

export async function fetchDesignImage(url) {
  const resp = await fetch(url);
  if (!resp.ok) {
    throw new Error(`Failed to fetch ${url}: ${resp.status}`);
  }
  const buf = await resp.arrayBuffer();
  return new Uint8Array(buf);
}
