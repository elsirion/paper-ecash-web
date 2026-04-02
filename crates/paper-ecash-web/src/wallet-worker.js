import init from "./paper-ecash-web.js";

await init();

self.postMessage("__ready__");
