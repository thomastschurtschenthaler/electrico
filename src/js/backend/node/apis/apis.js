require("./fs.js");
require("./net.js");
require("./child_process.js");
require("./http.js");
require("./crypto.js");
require("./node-fetch.js");
require("./stream.js");
require("./perf_hooks.js");
require("./undici.js");

window.__electrico.libs.constants={
    hasOwnProperty:(p) => {
        return false;
    }
}