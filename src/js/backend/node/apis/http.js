(function(){
    let http = {
        request(options, cb) {
            let req_events =  {};
            let resp_events =  {};
            let req = createCMDRequest(true);
            req.onreadystatechange = function() {
                if (this.readyState == 4) {
                    if (cb!=null) {
                        cb({
                            statusCode: req.status,
                            headers: req.getAllResponseHeaders().split("\r\n"),
                            on: (event, cb) => {
                                resp_events[event] = cb;
                            }
                        });
                    }
                    if (req.status == 200) {
                        if (resp_events["data"]!=null) {
                            resp_events["data"](req.response);
                        }
                    } else if (req_events["error"]!=null) {
                        req_events["error"]("error status "+req.status);
                    }
                }
            };
            req.error = function(e) {
                if (req_events["error"]!=null) {
                    req_events["error"]("error "+e);
                }
            }
            return {
                on: (event, cb) => {
                    req_events[event] = cb;
                },
                end: () => {
                    let cmd = {"api":"HTTP", "command": {"action": "Request", "options":options}};
                    let nodecmd = {"action":"Node", invoke:{"command":"Api", "data":JSON.stringify(cmd)}};
                    req.send(JSON.stringify(nodecmd));
                }
            }
        },
        Agent: class {
            constructor(opts) {
                console.log("http.Agent construct", opts);
            }
        }
    };
    window.__electrico.libs["node:https"] = http;
    window.__electrico.libs.https = http;
    window.__electrico.libs["node:http"] = http;
    window.__electrico.libs.http = http;
})();