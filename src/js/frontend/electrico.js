(function() {
    let initscript = function(document) {
        let ipcRenderer = null;
        let _XMLHttpRequest = XMLHttpRequest;
        var window=document.window;
        if (window.requestIdleCallback==null) {
            window.requestIdleCallback = (cb, op) => {
                setTimeout(cb, 0);
            }
        }
        let _encodeURI = window.encodeURI;
        window.encodeURI = (uri) => {
            let ix = uri.indexOf("://");
            if (ix>=0 && window.location.href.startsWith("fil://file/"+uri.substring(0, ix+3))) {
                uri = "fil://file/"+uri;
            }
            return _encodeURI(uri);
        }
        __init_shared(window);
        window.alert = (msg) => {
            const req = new XMLHttpRequest();
            req.open("POST", window.__create_protocol_url("ipc://ipc/send"), false);
            req.send(JSON.stringify({"action": "Alert", "message": msg}));
        }
        function sendIPC(request_id, nonce, async, ...args) {
            let Buffer = require('buffer').Buffer;
            const req = new _XMLHttpRequest();
            let data_blob = null;
            let channel = args[0];
            if (args.length>1 && (Buffer.isBuffer(args[1]) || args[1] instanceof Uint8Array)) {
                data_blob=args[1];
                args[1]={_electrico_buffer_id:request_id};
            }
            let action = JSON.stringify({"action":"PostIPC", "request_id":request_id, "nonce": nonce, "params":JSON.stringify(args)});
            req.open("POST", window.__create_protocol_url("ipc://ipc/ipc."+channel+(data_blob!=null?("?"+encodeURIComponent(action)):"")), async);
            req.send(data_blob!=null?data_blob:action);
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
                            req.open("POST", window.__create_protocol_url("ipc://ipc/send"), false);
                            req.send(JSON.stringify({"action":"GetProcessInfo", "nonce":nonce}));
                            _processInfo = JSON.parse(req.responseText);
                        } else {
                            _processInfo = {};
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
                    sendIPC(uuidv4(), this.nonce, true, ...args);
                }
                sendSync(...args) {
                    window.__electrico.ipcSyncResponse=null;
                    let req = sendIPC(uuidv4(), this.nonce, false, ...args);
                    if (req.readyState == 4 && req.status == 200) {
                        return JSON.parse(req.responseText);
                    }
                    console.error("sendSync request failed - timeout");
                    return null;
                }
                invoke(...args) {
                    return new Promise(resolve => {
                        let req = sendIPC(uuidv4(), this.nonce, true, ...args);
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
        electron = {
            __init_electrico_nonce: (nonce) => {
                return _electron(nonce);
            }
        }
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
            sendChannelMessage: (rid, channel, argumentsstr) => {
                setTimeout(()=>{
                    let args = JSON.parse(argumentsstr);
                    if (args.posted) {
                        let doCall = () => {
                            if (args.portid!=null) {
                                let port = window.__electrico.received_ports[args.portid];
                                port.postMessage(args.data);
                            } else {
                                let ports = args.ports.map((p) => {
                                    let channel = new MessageChannel();
                                    let port = channel.port1;
                                    port.onmessage = function(e) {
                                        sendIPC(uuidv4(), ipcRenderer.nonce, true, p.id, e.data);
                                    };
                                    window.__electrico.received_ports[p.id] = port;
                                    let _postMessage=channel.port2.postMessage;
                                    channel.port2.postMessage = (...args) => {
                                        _postMessage.bind(channel.port2)(...args);
                                    }
                                    return channel.port2;
                                });
                                if (args.fromWebContents) {
                                    let send = {"sender":ipcRenderer, "ports": ports};
                                    ipcRenderer.emit(channel, send, args.data);
                                } else {
                                    let event = new MessageEvent("message", {"ports":ports});
                                    event.data=args.data;
                                    ipcRenderer.emit(channel, event);
                                }
                            }
                        }
                        if (args.data._electrico_buffer_id!=null) {
                            const req = new XMLHttpRequest();
                            req.open("POST", window.__create_protocol_url("ipc://ipc/getdatablob"), true);
                            req.responseType = "arraybuffer";
                            req.send(JSON.stringify({"action":"GetDataBlob", "id":args.data._electrico_buffer_id}));
                            req.onreadystatechange = function() {
                                if (this.readyState == 4) {
                                    if (req.status == 200) {
                                        args.data = Buffer.from(req.response);
                                        doCall();
                                    }
                                }
                            };
                        } else {
                            doCall();
                        }
                    } else {
                        let send = {"sender":ipcRenderer, "ports": []};
                        ipcRenderer.emit(channel, send, ...args);
                    }
                }, 0);
                return "OK";
            },
            addArgument: (arg) => {
                window.__electrico.add_args.push(arg);
            }
        };
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
                    window.addEventListener=_addEventListener;
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
    initscript(document);
    var {Buffer} = require("buffer");
    window.Buffer=Buffer;
    window.addEventListener("DOMContentLoaded", ()=>{
        const req = new XMLHttpRequest();
        req.open("POST", window.__create_protocol_url("ipc://ipc/send"), true);
        req.send(JSON.stringify({"action": "DOMContentLoaded", "title": document.title}));
    })
})();
