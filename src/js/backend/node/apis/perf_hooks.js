(function() {
    const perf_hooks = {
        performance: {
            timeOrigin: (new Date()).getTime()
        }
    }
    window.__electrico.libs["perf_hooks"] = perf_hooks;
    window.__electrico.libs["node:perf_hooks"] = perf_hooks;
})();