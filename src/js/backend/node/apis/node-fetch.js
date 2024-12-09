(function() {
    let nodefetch = window.fetch;
    nodefetch.default = nodefetch;
    window.__electrico.libs["node-fetch"] = nodefetch;
})();