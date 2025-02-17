var __electrico_nonce=null;
(function() {
    let wkeys = ['location', 'screen', '__is_windows', '__http_protocol', '__electrico_nonce', 'crypto', 'createWindow', 'setTimeout', 'setInterval', 'clearTimeout','clearInterval', 'fetch', '__init_shared', '__init_require', 'btoa', 'atob', 'performance', 'TextDecoder', 'TextEncoder'];
    for (let k in window) {
        if (!wkeys.includes(k)) {
            window[k]=()=>{};
        } else {
            //console.log("excluded",k);
        }
    }
    let _setTimeout=window.setTimeout;
    let _setInterval=window.setInterval;
    let _clearTimeout=window.clearTimeout;
    let _clearInterval=window.clearInterval;
    window.setTimeout = function(f, t, ...args) {
        let timer = _setTimeout(f, t, ...args);
        return {
            _timer:timer,
            unref: () => {}
        }
    }
    window.setInterval = function(...args) {
        let timer = _setInterval(...args);
        return {
            _timer:timer,
            unref: () => {}
        }
    }
    window.clearTimeout = function(timer) {
        if (timer!=null) {
            _clearTimeout(timer._timer);
        }
    }
    window.clearInterval = function(timer) {
        if (timer!=null) {
            _clearInterval(timer._timer);
        }
    }
    window.setImmediate = function(cb) {
        return setTimeout(cb, 0);
    }
    window.clearImmediate = window.clearTimeout;
    
    window.__init_shared(window);
    function createCMDRequest(async, name) {
        const req = new XMLHttpRequest();
        if (async) {
            req.timeout=600000;
        }
        req.open("POST", window.__create_protocol_url("cmd://cmd/"+(name!=null?name:"execute")), async);
        return req;
    }
    window.createCMDRequest=createCMDRequest;
    let e_command = function(action) {
        return new Proxy({}, {
            get(target, call, rec) {
                return function(params, data_blob) {
                    let command; let async=false;
                    let bin = false;
                    if (call.endsWith("Bin")) {
                        bin=true;
                        call = call.substring(0, call.length-3);
                    }
                    if (call.startsWith("async")) {
                        async=true;
                        command=call.substring(5);
                    } else if (call.startsWith("sync")) {
                        command=call.substring(4);
                    } else {
                        command=call;
                    }
                    let command_parts = command.split("_");
                    
                    if (params==null) {
                        params={};
                    }
                    if (command_parts.length>2) {
                        params = {
                            "data": JSON.stringify({
                                "api": command_parts[1],
                                "command": {
                                    "action":command_parts[2],
                                    ...params
                                }
                            })
                        }
                    }
                    let body; let urlcmd=null;
                    let cmdjson = JSON.stringify({"action":action, invoke:{"command":command_parts[0], ...params}});
                    if (data_blob!=null) {
                        urlcmd=cmdjson;
                        body=data_blob;
                    } else {
                        body=cmdjson;
                    }
                    const req = new XMLHttpRequest();
                    req.open("POST", window.__create_protocol_url("cmd://cmd/"+action+"."+call+(urlcmd!=null?("?"+encodeURIComponent(urlcmd)):""), (!async && bin)), async);
                    if (async) {
                        req.timeout=600000;
                    }
                    if (bin) {
                        req.responseType = "arraybuffer";
                    }
                    req.send(body);
                    if (async) {
                        return {
                            then: cb => {
                                req.onreadystatechange = function() {
                                    if (this.readyState == 4) {
                                        if (req.status == 200) {
                                            cb(null, req.response);
                                        } else {
                                            cb(req.response, null);
                                        }
                                    }
                                };
                            }
                        }
                    } else {
                        if (req.status==200) {
                            return {r:req.response};
                        } else {
                            return {e:req.response};
                        }
                    }
                };
            }
        });
    }
    window.$e_node=e_command("Node");
    window.$e_electron=e_command("Electron");

    function createLogMsg(level, logmsg, logdata) {
        return {"params":{"level": level, "logmsg":logmsg, "logdata":JSON.stringify(logdata)}};
    }
    
    window.onerror = (event) => {
        window.__electrico.error=event;
    };
    let console_log = window.console.log;
    window._consolelog = window.console.error;
    let console_debug = window.console.debug;
    let console_error = window.console.error;
    let console_warn = window.console.warn;
    let console_trace = window.console.trace;
    window.console.log = (logmsg, ...logdata) => {
        console_log(logmsg, ...logdata);
        $e_node.asyncConsoleLog(createLogMsg("Info", logmsg+"", logdata));
    };
    window.console.info = window.console.log;
    window.console.debug = (logmsg, ...logdata) => {
        console_debug(logmsg, ...logdata);
        $e_node.asyncConsoleLog(createLogMsg("Debug", logmsg+"", logdata));
    };
    window.console.error = (logmsg, ...logdata) => {
        console_error(logmsg, ...logdata);
        for (let i=0; i<logdata.length; i++) {
            if (logdata[i] instanceof Error) logdata[i]=logdata[i].message;
        }
        $e_node.asyncConsoleLog(createLogMsg("Error", logmsg+"", logdata));
    };
    window.console.warn = (logmsg, ...logdata) => {
        console_warn(logmsg, ...logdata);
        $e_node.asyncConsoleLog(createLogMsg("Warn", logmsg+"", logdata));
    };
    window.console.trace = (logmsg, ...logdata) => {
        console_trace(logmsg, ...logdata);
        $e_node.asyncConsoleLog(createLogMsg("Trace", logmsg+"", logdata));
    };
    function sendIPCResponse(requestID, response) {
        let urlcmd = JSON.stringify({"action":"SetIPCResponse", "request_id":requestID});
        const req = new XMLHttpRequest();
        req.open("POST", window.__create_protocol_url("cmd://cmd/Frontend.SetIPCResponse?"+encodeURIComponent(urlcmd)), true);
        req.send(JSON.stringify(response));
    }
    function callChannel(timeout, browserWindowID, requestID, channel, ...args) {
        if (channel=="__electrico_protocol") {
            //console.log("callChannel - __electrico_protocol", args);
            window.__electrico.file_protocol[args[0]](requestID, {url:args[1]});
            return {returnValue:""};
        }

        let event = new Proxy({}, {
            get(target, prop, rec) {
                if (prop=="reply") {
                    return (response) => {
                        target.returnValue=response;
                    }
                } else if (prop=="sender") {
                    return window.__electrico.browser_window[browserWindowID].webContents;
                } else if (prop=="senderFrame") {
                    return {};
                }
                return target[prop];
            },
            set(target, prop, value) {
                if (prop=="returnValue") {
                    target.returnValue=value;
                    return true;
                }
            }
        });
        let ipcMain = require("electron").ipcMain;
        let resp = Promise.resolve(ipcMain.__callIpc(channel, event, ...args));
        setTimeout(()=>{
            resp.then(function(ret) {
                let response = event.returnValue!=null?event.returnValue:ret;
                if (response==undefined) response=null;
                event.returnValue = response;
                timeout.cleared = true;
                sendIPCResponse(requestID, response);
            }).catch((e) => {
                console.error("callChannel error", e);
                window.__electrico.error = e;
            });
        }, 0);
        return event;
    }
    window.__electrico={
        file_protocol: {},
        app_menu:{},
        module_paths: {},
        module_cache: {},
        ipc_connected: [],
        sendChannelMessage: (channel, args, data) => {
            if (channel == "ipc_connect") {
                window.__electrico.ipc_connected.push(args);
            } else if (channel.startsWith("ipc_")) {
                args = JSON.parse(args);
                window.__electrico.callIPCChannel(args.browser_window_id, args.request_id, channel.substring(4), args.params, data);
            } else if (channel.startsWith("cp_data_")) {
                let hook = "on_"+args;
                window.__electrico.child_process.callback[hook](channel.substring(8), data);
            } else if (channel.startsWith("cp_exit_")) {
                window.__electrico.child_process.callback.on_close(channel.substring(8), args*1);
            } else if (channel.startsWith("fsw_")) {
                args = JSON.parse(args);
                window.__electrico.fs_watcher.on_event(channel.substring(4), args.kind, args.filenames);
            } else if (channel.startsWith("net_start_")) {
                window.__electrico.net_server.callback.on_start(args, channel.substring(10));
            } else if (channel.startsWith("net_data")) {
                window.__electrico.net_server.callback.on_data(args, data);
            } else if (channel.startsWith("net_end")) {
                window.__electrico.net_server.callback.on_end(args);
            }
        },
        call: (f) => {
            setTimeout(f, 0);
            return "OK";
        },
        child_process: {
            callback: {
                on_stdout: (pid, bdata) => {
                    window.__electrico.child_process[pid].stdout.emit("data", bdata);
                },
                on_stderr: (pid, bdata) => {
                    window.__electrico.child_process[pid].stderr.emit("data", bdata);
                },
                on_close: (pid, exit_code) => {
                    window.__electrico.child_process[pid].stdout.emit("close");
                    window.__electrico.child_process[pid].stderr.emit("close");
                    window.__electrico.child_process[pid].emit("close", exit_code);
                    window.__electrico.child_process[pid].emit("exit", exit_code);
                    setTimeout(()=>{
                        delete window.__electrico.child_process[pid];
                    }, 100);
                }
            }
        },
        fs_watcher: {
            on_event: (wid, eventType, filenames) => {
                let files = filenames.split(";");
                for (let file of files) {
                    window.__electrico.fs_watcher[wid].on_event(eventType, file);
                }
            }
        },
        net_server: {
            callback: {
                on_start: (hook, id) => {
                    let server = window.__electrico.net_server[hook];
                    if (server!=null) {
                        server._connection_start(id);
                    }
                },
                on_data: (id, bdata) => {
                    let connection = window.__electrico.net_server[id];
                    if (connection!=null) {
                        connection.emit("data", bdata);
                    }
                },
                on_end: (id) => {
                    let connection = window.__electrico.net_server[id];
                    if (connection!=null) {
                        connection._connection_end(id);
                    }
                }
            }
        },
        net_client: {},
        app: {},
        libs: window.__electrico!=null?window.__electrico.libs:{},
        getLib: (mpath, nonce) => {
            let lib = window.__electrico.libs[mpath];
            return lib;
        },
        callback: {
            "BrowserWindowLoadfile": (id) => {
                let win = window.__electrico.browser_window[id]
                let cb = win.webContents.on['did-finish-load'];
                if (cb!=null) {
                    cb();
                }
            }
        },
        browser_window: {},
        loadMain: (main) => {
            window.__dirname = window.__electrico.appPath;
            window.__Import_meta = {url:window.__dirname};
            if (main==null) {
                return;
            }
            if (!main.startsWith("./")) {
                main = "./"+main;
            }
            require(main);
            //setTimeout(doLoadMain, 1000);
        },
        callIPCChannel: (browserWindowID, requestID, channel, argumentsstr, data) => {
            let arguments = JSON.parse(argumentsstr);
            
            let resp = null;
            delete window.__electrico.error;
            let timeout = {
                "cleared": false,
                trigger: function() {
                    if (!timeout.cleared) {
                        if (resp==null && window.__electrico.error!=null) {
                            console.error("callChannel script error", channel, window.__electrico.error);
                            delete window.__electrico.error;
                            sendIPCResponse(requestID, null);
                        } else {
                            setTimeout(timeout.trigger, 1000);
                        }
                    }
                }
            };
            setTimeout(timeout.trigger, 1000);
            if (arguments.length>0) {
                if (arguments[0]._electrico_buffer_id!=null) {
                    arguments[0] = data;
                } else if (data!=null) {
                    arguments[0].data = data;
                }
            }
            let event = callChannel(timeout, browserWindowID, requestID, channel, ...arguments);
            if (event.returnValue!=null) {
                resp=event.returnValue;
            }
        },
        getIPCChannelSyncResponse: () => {
            return window.__electrico.ipcChannelSyncResponse;
        },
        callAppOn: (event, windowID) => {
            if (event == "window-close") {
                let winids = windowID!=null?{[windowID]:windowID}:window.__electrico.browser_window;
                for (let winid in winids) {
                    let closeEvent = new CustomEvent("close");
                    let prevented = false;
                    closeEvent.preventDefault=() => {
                        prevented=true;
                    }
                    window.__electrico.browser_window[winid].emit("close", closeEvent);
                    if (!prevented) {
                        $e_electron.asyncBrowserWindowClose({"id":winid});
                    }
                }
            } else {
                const {app} = require('electron/main');
                app.emit(event, event, window.__electrico.browser_window[windowID]);
            }
        },
        menuSelected: (menuid) => {
            let item = window.__electrico.app_menu.idmapping[menuid];
            item.click(item, window.__electrico.browser_window[0], {});
        },
        domContentLoaded: (windowID) => {
            window.__electrico.browser_window[windowID].domContentLoaded();
        }
    };
    
    let EventEmitter = require('eventemitter3');
    class SerializationBuffer {
        constructor(clientid) {
            this.clientid=clientid;
            this.deserialize = (function(data, cb) {
                let msg = JSON.parse(data.msg);
                if (data.data_blob!=null) {
                    msg.data = data.data_blob;
                }
                cb(msg);
            }).bind(this);
            this.serialize = (function(msg, cb) {
                let Buffer = require('buffer').Buffer;
                let bindata = null;
                if (Buffer.isBuffer(msg.data)) {
                    bindata = msg.data;
                    delete msg.data;
                }
                let data = {msg:JSON.stringify(msg), portid:msg.portid, data_blob:bindata};
                cb(data);
            }).bind(this);
        }
    }
    window.__electrico.SerializationBuffer=SerializationBuffer;
    class ProcessPort extends EventEmitter {
        constructor(sender, sbuffer, id) {
            super();
            this.sender=sender;
            this.sbuffer=sbuffer;
            this.id = id;
            this.neutered_ports = {};
            this.received_ports = {};
            let _this=this;
            let __postMessage = (data, ports, portid, retry) => {
                ports = ports || [];
                if (!retry) {
                    ports.map((p) => {p.send_locked=true});
                    window.__call_queue("PPP"+_this.id, (msg)=>{
                        if (ports.filter(p=>{return p.connected_port!=null && p.connected_port.pending}).length>0) {
                            return false;
                        }
                        __postMessage(msg.data, msg.ports, msg.portid, true);
                        return true;
                    }, {data, ports, portid});
                    return;
                }
                let mports = ports.map((p) => {
                    delete p.send_locked;
                    if (p.neutered) {
                        console.error("port already neutered", p);
                        return null;
                    }
                    if (_this.pending_ports!=null) {
                        _this.pending_ports.push(p);
                        p.pending=true;
                    }
                    _this.neutered_ports[p.id] = p;
                    p.neutered=true;
                    p.on("message", (msg) => {
                        _this.__postMessage(msg.data, msg.ports, p.id);
                    });
                    let mport = {"id":p.id};
                    return mport
                });
                let msg = {portid:portid!=null?portid:_this.id, data:data, ports:mports};
                _this.sbuffer.serialize(msg, (data)=>{
                    _this.sender(data);
                });
            };
            this.postMessage = (data, ports, portid) => {
                __postMessage(data, ports, portid);
            }
            this.__postMessage=this.postMessage;
            this.onMessageReceived = (msg) => {
                msg.ports = msg.ports.map((p) => {
                    let rport = new ProcessPort(_this.sender, _this.sbuffer, p.id);
                    _this.received_ports[p.id] = rport;
                    return rport;
                });
                if (msg.portid!=null && _this.neutered_ports[msg.portid]!=null) {
                    _this.neutered_ports[msg.portid].postMessage(msg.data, msg.ports);
                } else {
                    window.__call_queue("PPE"+_this.id, (msg)=>{
                        let eport = msg.portid!=null?_this.received_ports[msg.portid]:_this;
                        if (eport==null) {
                            console.log("ProcessPort.onMessageReceived no port received for portid yet", msg.portid);
                            return false;
                        }
                        delete msg.portid;
                        if (eport.started) {
                            eport.emit("message", _this.flatten!=null?_this.flatten(msg):msg);
                        } else {
                            console.error("port not started!", eport);
                        }
                        return true;
                    }, msg);
                }
            }
            this.ondata = (data) => {
                _this.sbuffer.deserialize(data, (mjson)=> {
                    _this.onMessageReceived(mjson);
                });
            }
            this.start = (() => {
                this.started=true;
            }).bind(this);
            this.close = (() => {
                this.started=false;
            }).bind(this);
        }
    }
    window.__electrico.ProcessPort=ProcessPort;
    let init_fork = function(hook, clientid, envstr) {
        window.__electrico.parent_connected=true;
        let env = JSON.parse(envstr);
        for (let k in env) {
            process.env[k] = env[k];
        }
        let sbuffer = new SerializationBuffer(clientid);
        let parentPort = new ProcessPort((data) => {
            let action = {"action":"PostIPC", "http_id":"fork", "from_backend":true, "request_id":data.portid!=null?data.portid:"fork", "channel":clientid, "params":"["+data.msg+"]"};
            let action_msg = {"command": action, "data_blob":data.data_blob!=null};
            let ws_hook = hook;
            if (data.portid != null && window.__electrico.ipc_connected.includes(data.portid)) {
                ws_hook = null;
            }
            window.__ipc_websocket("ipc", false, null, ws_hook, (socket)=>{
                let msg = (new TextEncoder()).encode(JSON.stringify(action_msg));
                socket.send(msg);
                if (data.data_blob!=null) {
                    socket.send(data.data_blob);
                }
            });
        }, sbuffer);
        parentPort.start();
        /*window.__ipc_websocket("ipc", false, null, hook, (socket)=>{
            
        });*/
        
        window.__electrico.parentPort=parentPort;
        const ipcMain = require("electron").ipcMain;
        ipcMain.on(clientid, (function(e, msg) {
            parentPort.onMessageReceived(msg);
        }).bind(this));
        parentPort.send = function(message, sendHandle, options, callback) {
            if (callback==null) {
                callback=sendHandle;
                sendHandle=null; options=null;
            }
            parentPort.postMessage(message);
            if (callback!=null) {
                callback(null);
            }
        };
        let _parent_emit = parentPort.emit;
        parentPort.emit = function(...args) {
            _parent_emit.bind(parentPort)(...args);
            if (args[0]=="message") {
                process.emit("message", args[1].data);
            } else {
                process.emit(...args);
            }
        }
        let fork_hook = "ws://electrico.localhost:"+window.__http_protocol.http_port+"/"+window.__http_protocol.http_uid+"@asyncin/fork_"+process.pid;
        parentPort.postMessage({"hook":fork_hook});
    };
    window.__electrico.init_fork=init_fork;
    window.__electrico.contentsPostMessage = function(channel, message, ports) {
        ports = ports || [];
        $e_electron.asyncChannelSendMessage({"id":this._e_id, "channel":channel, "args":JSON.stringify(message)});
    };
    let _process = null;
    var process=new Proxy(new EventEmitter(), {
        get(target, prop, receiver) {
            if (prop=="stdout") {
                return {
                    write: (d) => {
                        console.log(d);
                    },
                    fd:0
                }
            }
            if (prop=="stderr") {
                return {
                    write: (d) => {
                        console.error(d);
                    },
                    fd:2
                }
            }
            if (prop=="cwd") {
                return () => {
                    return window.__electrico.appPath;
                }
            }
            if (prop=="kill") {
                return (pid, signal) => {
                    if (signal==0 && !window.__electrico.parent_connected) {
                        throw "parent gone";
                    }
                }
            }
            if (prop=="exit") {
                return () => {
                    console.error("process.exit called");
                    require('electron/main').app.quit();
                }
            }
            if (prop=="send") {
                return window.__electrico.parentPort.send.bind( window.__electrico.parentPort);
            }
            if (prop=="electronBinding") {
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
            }
            if (prop=="parentPort") {
                return window.__electrico.parentPort;
            }
            if (prop=="nextTick") {
                return (cb)=> {
                    setTimeout(cb, 0);
                };
            }
            if (_process==null) {
                let {r, e} = $e_node.syncGetProcessInfo();
                _process = JSON.parse(r);
                _process.version="v22.9.0";
                _process.execArgv=[];
                for (let k in _process) {
                    target[k] = _process[k];
                }
            }
            return target[prop];
        }
    });
    window.process=process;
})();
require("./node/node.js");
require("./electron/electron.js");
Error.captureStackTrace = (o)=> {
    let e = {
        getFileName: () => {
            return null;
        },
        getLineNumber: () => {
            return 1;
        },
        getColumnNumber: () => {
            return 1;
        },
        isEval: () => {
            return false;
        },
        getFunctionName: () => {
            return "dummy";
        }
    };
    o.stack=[e,e,e];
};
