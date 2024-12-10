var __electrico_nonce=null;
(function() {
    let wkeys = ['location', 'screen', '__is_windows', 'crypto', 'createWindow', 'setTimeout', 'setInterval', 'clearTimeout','clearInterval', 'fetch', '__init_shared', '__init_require', 'btoa', 'atob', 'performance'];
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
    window.setTimeout = function(...args) {
        let timer = _setTimeout(...args);
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
    window.__init_shared(window);
    function createCMDRequest(async, name) {
        const req = new XMLHttpRequest();
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
                    req.open("POST", window.__create_protocol_url("cmd://cmd/"+action+"."+call+(urlcmd!=null?("?"+encodeURIComponent(urlcmd)):"")), async);
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
        call: (f) => {
            setTimeout(f, 0);
            return "OK";
        },
        child_process: {
            callback: {
                on_stdout: (pid) => {
                    let Buffer = require('buffer').Buffer;
                    let {r, e} = $e_node.syncGetDataBlobBin({"id":pid+"stdout"});
                    if (e!=null) return;
                    let bdata = Buffer.from(r);
                    window.__electrico.child_process[pid].stdout.emit("data", bdata);
                },
                on_stderr: (pid) => {
                    let Buffer = require('buffer').Buffer;
                    let {r, e} = $e_node.syncGetDataBlobBin({"id":pid+"stderr"});
                    if (e!=null) return;
                    let bdata = Buffer.from(r);
                    window.__electrico.child_process[pid].stderr.emit("data", bdata);
                },
                on_close: (pid, exit_code) => {
                    window.__electrico.child_process[pid].emit("close", exit_code);
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
                on_data: (id) => {
                    let connection = window.__electrico.net_server[id];
                    if (connection!=null) {
                        let Buffer = require('buffer').Buffer;
                        let {r, e} = $e_node.syncGetDataBlobBin({"id":id});
                        let bdata = Buffer.from(r);
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
                //console.trace("BrowserWindowLoadfile done", id);
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
            //setTimeout(()=>{
                require(main);
            //}, 1000);
        },
        callIPCChannel: (browserWindowID, requestID, argumentsstr) => {
            setTimeout(()=>{
                let arguments = JSON.parse(argumentsstr);
                let channel = arguments[0];
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
                let doCall = () => {
                    let event = callChannel(timeout, browserWindowID, requestID, ...arguments);
                    if (event.returnValue!=null) {
                        resp=event.returnValue;
                    }
                };
                if (arguments.length>1 && arguments[1]._electrico_buffer_id!=null) {
                    $e_node.asyncGetDataBlobBin({"id":arguments[1]._electrico_buffer_id}).then((e, r)=>{
                        let Buffer = require('buffer').Buffer;
                        arguments[1] = Buffer.from(r);
                        doCall();
                    });
                } else {
                    doCall();
                }
            }, 0);
            return "OK";
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
                app.emit(event);
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
            this.LENGTH_BYTES = 8;
            this.buffers = [];
            this.actmsg=null;
            this.deserialize = (function(data, cb) {
                let Buffer = require('buffer').Buffer;
                this.buffers.push(data);
                let mdata = this.buffers.length>1?Buffer.concat(this.buffers):(this.buffers.length>0?this.buffers[0]:null);
                let pos=0; let cnt=0;
                while (true) {
                    cnt++;
                    if (this.actmsg==null) {
                        let p2 = pos+this.LENGTH_BYTES;
                        if (mdata.length>=p2) {
                            this.actmsg = {l_msgjson:Number(mdata.subarray(pos, pos+this.LENGTH_BYTES).readBigInt64LE())};
                            pos = p2;
                        } else {
                            break;
                        }
                    }
                    if (this.actmsg.msgjson==null) {
                        let p2 = pos+this.actmsg.l_msgjson;
                        if (mdata.length>=p2) {
                            try {
                                this.actmsg.msgjson=JSON.parse(mdata.subarray(pos, p2).toString());
                            } catch (e) {
                                console.error("SerializationBuffer.msgjson error parsing JSON", e);
                                throw e;
                            }
                            pos = p2;
                        } else {
                            break;
                        }
                    }
                    if (this.actmsg.msgjson.l_bindata!=null) {
                        let p2 = pos+this.actmsg.msgjson.l_bindata;
                        if (mdata.length>=p2) {
                            this.actmsg.msgjson.data=mdata.subarray(pos, p2);
                            delete this.actmsg.msgjson.l_bindata;
                            pos = p2;
                            cb(this.actmsg.msgjson);
                            this.actmsg=null;
                        } else {
                            break;
                        }
                    } else {
                        cb(this.actmsg.msgjson);
                        this.actmsg=null;
                    }
                }
                if (pos<mdata.length) {
                    this.buffers = [mdata.subarray(pos, mdata.length)];
                } else {
                    this.buffers = [];
                }
            }).bind(this);
            this.serialize = (function(msg, cb) {
                let Buffer = require('buffer').Buffer;
                let bindata = null;
                if (Buffer.isBuffer(msg.data)) {
                    bindata = msg.data;
                    msg.l_bindata = bindata.length;
                    delete msg.data;
                }
                let msgjson = Buffer.from(JSON.stringify(msg));
                let msglength = Buffer.alloc(this.LENGTH_BYTES);
                msglength.writeBigInt64LE(BigInt(msgjson.length));
                /*let bytes = msgjson.length+"";
                let msglength = Buffer.from(bytes + (new Array(this.LENGTH_BYTES-bytes.length+1).join(" ")));*/
                
                let data = null;
                if (bindata!=null) {
                    data = Buffer.concat([msglength, msgjson, bindata]);
                } else {
                    data = Buffer.concat([msglength, msgjson]);
                }
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
                ports.map((p) => {p.send_locked=true});
                if (ports.filter(p=>{return p.connected_port!=null && p.connected_port.pending}).length>0) {
                    setTimeout(()=>{__postMessage(data, ports, portid, true)}, 50);
                    return;
                }
                if (retry) {
                    setTimeout(()=>{__postMessage(data, ports, portid)}, 100);
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
                    return {"id":p.id};
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
                    let emit = () => {
                        let eport = msg.portid!=null?_this.received_ports[msg.portid]:_this;
                        if (eport==null) {
                            console.log("ProcessPort.onMessageReceived no port received for portid yet", msg.portid);
                            setTimeout(emit, 100);
                            return;
                        }
                        delete msg.portid;
                        if (eport.started) {
                            eport.emit("message", _this.flatten!=null?_this.flatten(msg):msg);
                        } else {
                            console.error("port not started!");
                        }
                    }
                    emit();
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
        //console.log("init_fork", hook, clientid, envstr);
        let env = JSON.parse(envstr);
        for (let k in env) {
            process.env[k] = env[k];
        }
        let { createConnection } = require('net');
        let con = createConnection(hook, () => {
            console.warn("init_fork clientid", clientid); con.write(clientid);
        });
        con.on("error", (e)=>{
            console.error("init_fork client connection error", e);
        });
        con.on('end', () => {
            console.log("init_fork client connection ended");
        });
        let sbuffer = new SerializationBuffer(clientid);
        let parentPort = new ProcessPort((data) => {
            con.write(data);
        }, sbuffer);
        window.__electrico.parentPort=parentPort;
        parentPort.start();
        con.on('data', (data) => {
            parentPort.ondata(data);
        });
    };
    window.__electrico.init_fork=init_fork;
    
    let mainIPCServer =  {
        init: function() {
            const {app} = require('electron/main');

            let { createServer } = require('net');
            server = createServer();
            let err = (e)=>{
                console.error("mainIPCServer server error", e);
            };
            server.on('error', err);
            this.hook = app.getPath("temp")+"/"+window.__uuidv4();
            console.warn("mainIPCServer hook", this.hook, new Date());
            if (this.hook.length>100) {
                this.hook = this.hook.substring(0, 100);
            }
            let _this=this;
            server.listen(this.hook, () => {
                server.removeListener('error', err);
                server.on("connection", (con)=>{
                    console.log("mainIPCServer got connection", con, new Date());
                    let proc = null;
                    con.on('end', () => {
                        console.log("mainIPCServer client connection ended", proc);
                    });
                    con.on('data', (data) => {
                        if (proc==null) {
                            let clientid = data.toString();
                            console.log("mainIPCServer got clientid", clientid, new Date());
                            proc = _this.procs[clientid];
                            if (proc==null) {
                                console.error("mainIPCServer server not connected to process - clientid:", clientid);
                            } else {
                                proc.con=con;
                                proc.forked();
                            }
                            return;
                        }
                        proc.ondata(data);
                    });
                    con.on("error", (e)=>{
                        console.error("mainIPCServer server error", e);
                    });
                });
            });
        },
        procs: {},
        connect: function (proc) {
            this.procs[proc.clientid]=proc;
        }/*,
        send: function (proc, data) {
            this.procs[proc.clientid].con.write(data);
        }*/
    };
    setTimeout(()=>{mainIPCServer.init();}, 0);
    window.__electrico.mainIPCServer=mainIPCServer;

    window.__electrico.contentsPostMessage = function(channel, message, ports) {
        ports = ports || [];
        $e_electron.asyncChannelSendMessage({"id":this._e_id, "rid":window.__uuidv4(), "channel":channel, "args":JSON.stringify(message)});
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
            if (prop=="exit") {
                return () => {
                    console.log("process.exit called");
                }
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
            if (_process==null) {
                let {r, e} = $e_node.syncGetProcessInfo();
                _process = JSON.parse(r);
                _process.env['VSCODE_DEV']=1;
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
