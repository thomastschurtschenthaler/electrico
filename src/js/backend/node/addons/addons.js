require("./sqlite.js");
require("./spdlog.js");
require("./pty.js");
window.__electrico.libs["bindings"] = (module) => {
    return require(module);
};