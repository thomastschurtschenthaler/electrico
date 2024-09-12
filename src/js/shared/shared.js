(function() {
    function init_shared (window, backend) {
        window.__init_require(window);
        if (backend) {
            function createCMDRequest(async) {
                const req = new XMLHttpRequest();
                req.open("POST", "cmd://cmd/execute", async);
                return req;
            }
            window.createCMDRequest=createCMDRequest;
        }
    }
    window.__init_shared = init_shared;
})();