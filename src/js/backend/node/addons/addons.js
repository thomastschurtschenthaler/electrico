require("./sqlite.js");
require("./spdlog.js");
require("./pty.js");
window.__electrico.libs["bindings"] = (module) => {
    return require(module);
};
window.__electrico.libs["@parcel"] = {
    watcher: {
        subscribe:(path, cb) => {
            console.log("parcelWatcher.subscribe", path);
        },
        getEventsSince:() => {
            console.log("parcelWatcher.getEventsSince");
            return [];
        },
        writeSnapshot:() => {
            console.log("parcelWatcher.writeSnapshot");
        }
    }
}
