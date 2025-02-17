(function() {
    let initscript = function(document, __electrico_nonce) {
        let ipcRenderer = null;
        let _XMLHttpRequest = XMLHttpRequest;
        var window=document.window;
        if (window.requestIdleCallback==null) {
            let cbs = {}; let cbnr=0;
            window.requestIdleCallback = (cb, op) => {
                let to = (op!=null && op.timeout!=null)?op.timeout:0;
                cbnr++;
                cbs[cbnr] = true;
                function timout(nr, t) {
                    const end = Date.now() + 15; // one frame at 64fps
                    const deadline = {
                        didTimeout: true,
                        timeRemaining() {
                            return Math.max(0, end - Date.now());
                        }
                    };
                    setTimeout(()=>{
                        if (cbs[nr]) {
                            delete cbs[nr];
                            cb(Object.freeze(deadline));
                        }
                    }, t);
                }
                timout(cbnr, to)
                return cbnr;
            }
            window.cancelIdleCallback = (tid) => {
                delete cbs[tid];
            }
        }
        function create_ipc_url(path) {
            if (window.location.protocol=="http:" || window.location.protocol=="https:") {
                return "electrico-file://file/electrico-ipc/"+path;
            }
            return window.location.protocol+"//"+window.location.host+"/electrico-ipc/"+path;
        }
        window.__create_ipc_url = create_ipc_url;
        __init_shared(window);
        window.alert = (msg) => {
            const req = new XMLHttpRequest();
            req.open("POST", window.__create_protocol_url(create_ipc_url("send")), false);
            req.send(JSON.stringify({"action": "Alert", "message": msg}));
        }
        

        function sendIPC(request_id, nonce, async, ws, channel, ...args) {
            let Buffer = require('buffer').Buffer;
            
            let data_blob = null;
            if (args.length>0 && (Buffer.isBuffer(args[0]) || args[0] instanceof Uint8Array)) {
                data_blob=args[0];
                args[0]={_electrico_buffer_id:request_id};
            }
            let action = JSON.stringify({"action":"PostIPC", "request_id":request_id, "data_blob":data_blob!=null, "nonce": nonce, "channel":channel, "params":JSON.stringify(args)});
            if (ws) {
                window.__ipc_websocket("ipc", false, nonce, null, (socket)=>{
                    let msg = (new TextEncoder()).encode(action);
                    socket.send(msg);
                    if (data_blob!=null) {
                        socket.send(data_blob);
                    }
                });
                return;
            }
            let req = new _XMLHttpRequest();
            if (async) {
                req.timeout=600000;
            }
            req.open("POST", window.__create_protocol_url(create_ipc_url("ipc."+channel+(data_blob!=null?("?"+encodeURIComponent(action)):""))), async);
            req.send(data_blob!=null?data_blob:action);
            if (!async && req.status!=200) {
                console.log("sendIPC sync error", req.status, channel);
            }
            return req;
        }
        let uuidv4 = window.__uuidv4;
        function processi(nonce) {
            let _processInfo=null;
            return new Proxy({}, {
                get(target, prop, receiver) {
                    if (_processInfo==null) {
                        if (nonce!=null) {
                            const req = new _XMLHttpRequest();
                            req.open("POST", window.__create_protocol_url(create_ipc_url("send")), false);
                            req.send(JSON.stringify({"action":"GetProcessInfo", "nonce":nonce}));
                            _processInfo = JSON.parse(req.responseText);
                        } else {
                            _processInfo = {"env":{}};
                        }
                    }
                    if (prop=="on") {
                        return (event, f) => {
                            //console.log("process on", event, f);
                        }
                    } else if (prop=="electronBinding") {
                        //console.log("electronBinding");
                        return (nodeversion) => {
                            return {
                                getHiddenValue: (w) => {
                                    //console.log("getHiddenValue");
                                    return "electrico";
                                },
                                isViewApiEnabled: () => {
                                    true;
                                }
                            }
                        }
                    } else if (prop=="argv") {
                        let args = _processInfo["argv"];
                        return args.concat(window.__electrico.add_args);
                    }
                    return _processInfo[prop];
                }
            });
        }
        window.process = processi(__electrico_nonce);
       
        let _electron_i = {};
        
        let _electron = function(nonce) {
            if (_electron_i[nonce]!=null) {
                return _electron_i[nonce];
            }
            let EventEmitter = require('eventemitter3');
            class IpcRendererCls extends EventEmitter {
                constructor(nonce) {
                    super();
                    this.nonce=nonce;
                }
                send(...args) {
                    sendIPC(uuidv4(), this.nonce, true, true, ...args);
                }
                sendSync(...args) {
                    window.__electrico.ipcSyncResponse=null;
                    let req = sendIPC(uuidv4(), this.nonce, false, false, ...args);
                    if (req.readyState == 4 && req.status == 200) {
                        return JSON.parse(req.responseText);
                    }
                    console.error("sendSync request failed - timeout");
                    return null;
                }
                invoke(...args) {
                    return new Promise(resolve => {
                        let req = sendIPC(uuidv4(), this.nonce, true, false, ...args);
                        req.onreadystatechange = function() {
                            if (this.readyState == 4) {
                                if (req.status == 200) {
                                    resolve(JSON.parse(req.responseText));
                                } else {
                                    console.error("invoke async response failed - timeout");
                                    resolve(null);
                                }
                            }
                        };
                    });
                }
            }
            let _ipcRenderer = new IpcRendererCls(nonce);
            if (nonce!=null && nonce.length>0 && ipcRenderer==null) {
                ipcRenderer = _ipcRenderer
            }
            _electron_i[nonce] = {
                ipcRenderer: _ipcRenderer,
                contextBridge: {
                    exposeInMainWorld: (method, fun) => {
                        window[method] = fun;
                    }
                },
                webFrame: {
                    setZoomLevel: (level) => {
                        console.log("setZoomLevel", level);
                        //TODO
                    }
                },
                webUtils: {
                    getPathForFile: (file) => {
                        console.log("getPathForFile", file);
                        return file;
                    }
                }
            }
            return _electron_i[nonce];
        };
        const electron = {
            __init_electrico_nonce: (nonce) => {
                return _electron(nonce);
            }
        };
        const remote_hooks = {}; 
        window.__electrico={
            module_paths: {},
            module_cache: {},
            channel: {},
            received_ports: {},
            add_args: [],
            libs: {
                "electron":electron,
            },
            replaceImports: (script) => {
                return script.replaceAll(/\import  *([^ ]*) *from *([^{ ,;,\r, \n}]*)/g, "var $1 = __import($2)");
            },
            getLib: (mpath, nonce) => {
                let lib = window.__electrico.libs[mpath];
                if (lib!=null && nonce!=null && lib.__init_electrico_nonce!=null) {
                    lib = lib.__init_electrico_nonce(nonce);
                }
                return lib;
            },
            sendChannelMessage: (channel, arguments, data) => {
                let args = (typeof arguments == 'object')?arguments:JSON.parse(arguments);
                if (data!=null) args.data = data;
                if (channel=="__posted_remote_connect_hook") {
                    remote_hooks[args.data.id] = args.data;
                    const remote_out_hook = args.data.hook.replace("asyncin", "asyncout/"+args.data.remote_id)+"_out";
                    const local_port_id = args.data.id;
                    window.__ipc_websocket("ipc", true, null, remote_out_hook, 
                        window.__ipc_websocket_messagehandler((channel, args, data) => {
                            const msg = JSON.parse(JSON.parse(args).params)[0];
                            msg.portid = local_port_id;
                            msg.posted = true;
                            window.__electrico.sendChannelMessage(channel, msg, data);
                        }),
                        (socket)=>{});
                    return;
                }
                if (args.posted) {
                    if (args.portid!=null) {
                        window.__call_queue(args.portid, (args)=>{
                            let port = window.__electrico.received_ports[args.portid];
                            if (port!=null) {
                                port.postMessage(args.data);
                            }
                            return port!=null;
                        }, args);
                    } else {
                        let ports = args.ports.map((p) => {
                            let mchannel = new MessageChannel();
                            let port = mchannel.port1;
                            port.onmessage = function(e) {
                                if (remote_hooks[p.id]!=null) {
                                    let data_blob = null; let data = e.data;
                                    if (Buffer.isBuffer(data) || data instanceof Uint8Array) {
                                        data_blob=e.data;
                                        data={};
                                    }
                                    let msg = {portid:remote_hooks[p.id].remote_id, data:data, ports:[]};
                                    let action = {"action":"PostIPC", "http_id":"remote", "from_backend":true, "request_id":"remote", "nonce": null, "channel":remote_hooks[p.id].clientid, "params":JSON.stringify([msg])};
                                    let action_msg = {"command": action, "data_blob":data_blob!=null};
                                    window.__ipc_websocket("ipc", false, null, remote_hooks[p.id].hook+"_in", (socket)=>{
                                        let msg = (new TextEncoder()).encode(JSON.stringify(action_msg));
                                        socket.send(msg);
                                        if (data_blob!=null) {
                                            socket.send(data_blob);
                                        }
                                    });
                                } else {
                                    sendIPC(uuidv4(), ipcRenderer.nonce, true, true, p.id, e.data);
                                }  
                            };
                            window.__electrico.received_ports[p.id] = port;
                            let _postMessage=mchannel.port2.postMessage;
                            mchannel.port2.postMessage = (...args) => {
                                _postMessage.bind(mchannel.port2)(...args);
                            }
                            return mchannel.port2;
                        });
                        if (args.fromWebContents) {
                            let send = {"sender":ipcRenderer, "ports": ports};
                            //console.error("fromWebContents", channel, send, args);
                            ipcRenderer.emit(channel, send, args.data);
                        } else {
                            let event = new MessageEvent("message", {"ports":ports});
                            event.data=args.data;
                            ipcRenderer.emit(channel, event);
                        }
                    }
                } else {
                    let send = {"sender":ipcRenderer, "ports": []};
                    ipcRenderer.emit(channel, send, ...args);
                }
            },
            addArgument: (arg) => {
                window.__electrico.add_args.push(arg);
            }
        };
        function getProtocol() {
            let loc = window.location.href;
            let i1 = loc.indexOf("@");
            if (i1<0) return null;
            let i2 = loc.indexOf("/", i1+1);
            if (i2<0) return null;
            return loc.substring(i1+1, i2);
        }
        let _addEventListener = window.addEventListener;
        //setTimeout(()=>{
            window.__electrico_preload(document, {
                before: (nonce) => {
                    window.addEventListener = (e, h) => {
                        _addEventListener(e, (e)=>{
                            let process=processi(nonce);
                            let he = "("+h.toString()+")(e)";
                            eval(he);
                        })
                    };
                },
                after: () => {   
                    window.addEventListener = (e, h, o) => {
                        if (e=="message") {
                            let _h=h;
                            h = function(e) {
                                _h(new Proxy(e, {
                                    get(target, prop, receiver) {
                                        if (prop=="origin") {
                                            let i1 = e.origin.indexOf("://");
                                            let i2 = e.origin.indexOf(".localhost", i1);
                                            return window.__custom_iframe_protocol+e.origin.substring(i1, i2);
                                        }
                                        return target[prop];
                                    }
                                }));
                            }
                        }
                        _addEventListener(e, h, o);
                    };
                    window.process=processi(null);
                }
            });
        //}, 1000);
        
        let start = (new Date()).getTime();
        let init_iframes = (nonce)=>{
            let iframes = document.querySelectorAll("iframe");
            if (iframes.length>0) {
                for (let i=0; i<iframes.length; i++) {
                    try {
                        let framewindow = iframes[i].contentWindow;
                        framewindow.initscript=initscript;
                        framewindow.__electrico_preload=window.__electrico_preload;
                        framewindow.__init_require=window.__init_require;
                        let _addEventListener = framewindow.addEventListener;
                        let domLoadedHandlers = [];
                        framewindow.addEventListener = (event, handler)=>{
                            if (event=="DOMContentLoaded") {
                                domLoadedHandlers.push(handler);
                            } else {
                                _addEventListener(event, handler);
                            }
                        }
                        __electrico_nonce=nonce;
                        framewindow.eval("window.document.window=window; window.initscript(window.document);");
                        __electrico_nonce='';
                        setTimeout(()=>{
                            console.trace("calling iframe domLoadedHandlers preload handlers", framewindow.document.documentElement);
                            for (let h of domLoadedHandlers) {
                                h();
                            }
                        }, 0);
                    } catch (e) {
                        console.error("electrico frame init error", e);
                    }
                }
            } else if ((new Date()).getTime()-start<2000) {
                setTimeout(()=>{init_iframes(nonce);}, 200);
            }
        };
        init_iframes(__electrico_nonce);
    }
    document.window=window;
    initscript(document, __electrico_nonce);
    if (window.__http_protocol!=null) {
        require("./quirks.js");
    }
    var {Buffer} = require("buffer");
    window.Buffer=Buffer;
    window.addEventListener("DOMContentLoaded", ()=>{
        let url = null;
        if (window.location.protocol=="http:" || window.location.protocol=="https:") {
            url = "electrico-file://file/electrico-ipc/send";
        } else {
            url = window.location.protocol+"//"+window.location.host+"/electrico-ipc/send"
        }
        const req = new XMLHttpRequest();
        req.open("POST", window.__create_protocol_url(url), true);
        req.send(JSON.stringify({"action": "DOMContentLoaded", "title": document.title}));
    })
})();
