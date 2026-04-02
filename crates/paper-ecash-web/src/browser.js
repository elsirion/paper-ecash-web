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

export async function fetchDesignImage(url) {
  const resp = await fetch(url);
  if (!resp.ok) {
    throw new Error(`Failed to fetch ${url}: ${resp.status}`);
  }
  const buf = await resp.arrayBuffer();
  return new Uint8Array(buf);
}
