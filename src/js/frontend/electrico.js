(function() {
    let initscript = function(document) {
        let ipcRenderer = null;
        let _XMLHttpRequest = XMLHttpRequest;
        var window=document.window;
        __init_shared(window);
        window.alert = (msg) => {
            const req = new XMLHttpRequest();
            req.open("POST", window.__create_protocol_url("ipc://ipc/send"), false);
            req.send(JSON.stringify({"action": "Alert", "message": msg}));
        }
        function sendIPC(request_id, nonce, async, ...args) {
            const req = new _XMLHttpRequest();
            req.open("POST", window.__create_protocol_url("ipc://ipc/send"), async);
            req.send(JSON.stringify({"action":"PostIPC", "request_id":request_id, "nonce": nonce, "params":JSON.stringify(args)}));
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
                        return [];
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
            sendChannelMessage: (argumentsstr) => {
                setTimeout(()=>{
                    let sep_channel = argumentsstr.indexOf("@");
                    let channel = argumentsstr.substring(0, sep_channel);
                    let args = JSON.parse(argumentsstr.substring(sep_channel+1, argumentsstr.length));
                    ipcRenderer.emit(channel, {}, ...args);
                }, 0);
                return "OK";
            }
        };
        let _addEventListener = window.addEventListener;
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
        //setTimeout(()=>{window.__electrico_preload(document);}, 1000);
        
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
