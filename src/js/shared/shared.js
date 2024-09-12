(function() {
    function init_shared (window, backend) {
        function getCircularReplacer() {
            const ancestors = [];
            return function (key, value) {
              if (typeof value !== "object" || value === null) {
                return value;
              }
              while (ancestors.length > 0 && ancestors.at(-1) !== this) {
                ancestors.pop();
              }
              if (ancestors.includes(value)) {
                return null;
              }
              ancestors.push(value);
              return value;
            };
        }
        let _stringify = JSON.stringify;
        JSON.stringify = (obj, r) => {
            return _stringify(obj, r!=null?r:getCircularReplacer());
        };
        window.__create_protocol_url = (url) => {
            if (window.__is_windows) {
                let ix = url.indexOf(":");
                url = "http://"+url.substring(0,ix)+"."+url.substring(ix+3);
            }
            return url;
        };
        window.__init_require(window);
        if (backend) {
            function createCMDRequest(async) {
                const req = new XMLHttpRequest();
                req.open("POST", window.__create_protocol_url("cmd://cmd/execute"), async);
                return req;
            }
            window.createCMDRequest=createCMDRequest;
        }
    }
    window.__init_shared = init_shared;
})();