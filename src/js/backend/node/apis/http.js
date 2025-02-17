(function() {
    let EventEmitter = require('eventemitter3');
    let {createServer, Server} = require("node:net");
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
        },
        Server: Server,
        createServer: function(options, listener) {
            if (listener==null) {
                listener = options; options=null;
            }
            
            let server = createServer(options, listener);
            let _listen = server.listen;
            server.listen = ((hook, cb) => {
                if (hook>=0 && hook<65536) {
                    if (listener!=null) {
                        server.removeListener("connection", listener);
                        server.on("request", listener);
                    }
                    server.on("connection", c=>{
                        console.log("http server got connection", c);
                        let http_response = new class extends EventEmitter {
                            constructor() {
                                super();
                                this.headers={};
                                this.headersWritten=false;
                                this.statusCode=200;
                                this.writeHead = (function(statusCode, statusMessage, headers) {
                                    if (headers==null) {
                                        headers=statusMessage;
                                        statusMessage=null;
                                    }
                                    headers = headers || [];
                                    for (let k in headers) {
                                        headers[k] = headers[k]+"";
                                    }
                                    c.write(JSON.stringify({'statusCode':statusCode+"", 'statusMessage':statusMessage, 'headers':headers}));
                                    this.headersWritten=true;
                                }).bind(this);
                                this._writeHead = (function() {
                                    if (this.headersWritten) return;
                                    c.write(JSON.stringify({'statusCode':this.statusCode+"", 'statusMessage':this.statusCode+"", 'headers':this.headers}));
                                    this.headersWritten=true;
                                }).bind(this);
                                this.write = (function(data, encoding, cb) {
                                    this._writeHead();
                                    c.write(data, encoding, cb);
                                }).bind(this);
                                this.end = (function(data, encoding, cb) {
                                    this._writeHead();
                                    if (data==null) data = "";
                                    c.write(data, encoding, cb, true);
                                }).bind(this);
                                this.resume = (function() {
                                    this.end();
                                }).bind(this);
                                this.setHeader = (function(k, v) {
                                    this.headers[k]=v+"";
                                }).bind(this);
                                this.getHeader = (function(k) {
                                    return this.headers[k];
                                }).bind(this);
                                this.removeHeader = (function(k) {
                                    return delete this.headers[k];
                                }).bind(this);
                            }
                        };
                        c._http_request=null;
                        c.on("data", d=>{
                            if (c._http_request==null) {
                                let headers = JSON.parse(d.toString());
                                c._http_request = new EventEmitter();
                                for (let k in headers) {
                                    c._http_request[k] = headers[k];
                                }
                                c._http_request.resume = ()=>{

                                };
                                server.emit("request", c._http_request, http_response);
                            } else {
                                c._http_request.emit("data", d);
                            }
                        });
                        c.on("end", ()=>{
                            c._http_request.emit("close");
                        });
                    });
                }
                _listen.bind(server)(hook, null, true);
                server.address = (function() {
                    return {port:this.id*1}
                }).bind(server);
                return server;
            }).bind(server);
            return server;
        }
    };
    window.__electrico.libs["node:https"] = http;
    window.__electrico.libs.https = http;
    window.__electrico.libs["node:http"] = http;
    window.__electrico.libs.http = http;
})();