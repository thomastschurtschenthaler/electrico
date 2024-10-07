var __electrico_nonce=null;
(function() {
    let wkeys = ['location', 'screen', '__is_windows', 'createWindow', 'setTimeout', 'fetch', '__init_shared', '__init_require', 'btoa', 'atob'];
    for (let k in window) {
        if (!wkeys.includes(k)) {
            window[k]=()=>{};
        } else {
            //console.log("excluded",k);
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
                    if (call.startsWith("async")) {
                        async=true;
                        command=call.substring(5);
                    } else if (call.startsWith("sync")) {
                        command=call.substring(4);
                    } else {
                        command=call;
                    }
                    if (params==null) {
                        params={};
                    }
                    let body; let urlcmd=null;
                    let cmdjson = JSON.stringify({"action":action, invoke:{"command":command, ...params}});
                    if (data_blob!=null) {
                        urlcmd=cmdjson;
                        body=data_blob;
                    } else {
                        body=cmdjson;
                    }
                    const req = new XMLHttpRequest();
                    req.open("POST", window.__create_protocol_url("cmd://cmd/"+action+"."+call+(urlcmd!=null?("?"+encodeURIComponent(urlcmd)):"")), async);
                    req.send(body);
                    if (async) {
                        return {
                            then: cb => {
                                req.onreadystatechange = function() {
                                    if (this.readyState == 4) {
                                        if (req.status == 200) {
                                            cb(null, req.responseText);
                                        } else {
                                            cb(req.responseText, null);
                                        }
                                    }
                                };
                            }
                        }
                    } else {
                        if (req.status==200) {
                            return {r:req.responseText};
                        } else {
                            return {e:req.responseText};
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
    var SenderCls=null;
    function callChannel(timeout, browserWindowID, requestID, channel, ...args) {
        if (SenderCls==null) {
            let EventEmitter = require('eventemitter3');
            class _SenderCls extends EventEmitter {
                getLastWebPreferences(){
                    return {
                        enableRemoteModule: true
                    }
                }
                getOwnerBrowserWindow () {
                    return window.__electrico.browser_window[browserWindowID];
                }
            }
            SenderCls = _SenderCls;
        }
        let event = new Proxy({}, {
            get(target, prop, rec) {
                if (prop=="reply") {
                    return (response) => {
                        target.returnValue=response;
                    }
                } else if (prop=="sender") {
                    return new SenderCls();
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
        let resp = Promise.resolve(window.__electrico.channel[channel](event, ...args));
        setTimeout(()=>{
            resp.then(function(ret) {
                let response = event.returnValue!=null?event.returnValue:ret;
                if (response==undefined) response=null;
                event.returnValue = response;
                timeout.cleared = true;
                const req = createCMDRequest(true, "Frontend.SetIPCResponse");
                req.send(JSON.stringify({"action":"SetIPCResponse", "request_id":requestID, "params": JSON.stringify(response)}));
            }).catch((e) => {
                console.error("callChannel error", e);
                window.__electrico.error = e;
            });
        }, 0);
        return event;
    }
    function exctractIPCParams(argumentsstr) {
        let sep_browserwindow = argumentsstr.indexOf("@");
        let sep_requestid = argumentsstr.indexOf("@@");
        return {
            browserWindowID: argumentsstr.substring(0, sep_browserwindow),
            requestID: argumentsstr.substring(sep_browserwindow+1, sep_requestid),
            arguments: JSON.parse(argumentsstr.substring(sep_requestid+2, argumentsstr.length))
        }
    }
    function wrapNodeInvoke(invoke) {
        return {"action":"Node", invoke:invoke};
    }
    window.__electrico={
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
                    let {r, e} = $e_node.syncGetDataBlob({"id":pid});
                    let bdata = Buffer.from(r);
                    let cb = window.__electrico.child_process[pid].stdout_on['data'];
                    if (cb!=null) {
                        cb(bdata);
                    }
                },
                on_stderr: (pid, data) => {
                    let Buffer = require('buffer').Buffer;
                    let {r, e} = $e_node.syncGetDataBlob({"id":pid});
                    let bdata = Buffer.from(r);
                    let cb = window.__electrico.child_process[pid].stderr_on['data'];
                    if (cb!=null) {
                        cb(bdata);
                    }
                },
                on_close: (pid, exit_code) => {
                    let cb = window.__electrico.child_process[pid].on['close'];
                    if (cb!=null) {
                        try {
                            cb(exit_code);
                        } catch (e) {
                            console.log("child_process.on_close", e);
                        }
                    }
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
                        let {r, e} = $e_node.syncGetDataBlob({"id":id});
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
        channel:{},
        browser_window: {},
        loadMain: (main) => {
            window.__dirname = window.__electrico.appPath+(main.indexOf("/")>=0?("/"+main.substring(0, main.indexOf("/"))):"");
            window.__Import_meta = {url:window.__dirname};
            if (!main.startsWith("./")) {
                main = "./"+main;
            }
            //setTimeout(()=>{
                require(main);
            //}, 1000);
        },
        callIPCChannel: (argumentsstr) => {
            let p = exctractIPCParams(argumentsstr);
            let channel = p.arguments[0];
            let resp = null;
            delete window.__electrico.error;
            let timeout = {
                "cleared": false,
                trigger: function() {
                    if (!timeout.cleared) {
                        if (resp==null && window.__electrico.error!=null) {
                            console.error("callChannel script error", channel, window.__electrico.error);
                            delete window.__electrico.error;
                            const req = createCMDRequest(true, "Frontend.SetIPCResponse");
                            req.send(JSON.stringify({"action":"SetIPCResponse", "request_id":p.requestID, "params": JSON.stringify(null)}));
                        } else {
                            setTimeout(timeout.trigger, 1000);
                        }
                    }
                }
            };
            setTimeout(timeout.trigger, 1000);
            setTimeout(()=>{
                let event = callChannel(timeout, p.browserWindowID, p.requestID, ...p.arguments);
                if (event.returnValue!=null) {
                    resp=event.returnValue;
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
                        const req = createCMDRequest(true, "Frontend.BrowserWindowClose");
                        req.send(JSON.stringify(window.__electrico.wrapInvoke({"command":"BrowserWindowClose", "id":winid}))); 
                    }
                }
            } else {
                const {app} = require('electron/main');
                app.emit(event);
            }
        },
        menuSelected: (menuid) => {
            let item = window.__electrico.app_menu.idmapping[menuid];
            item.click(item);
        },
        domContentLoaded: (windowID) => {
            window.__electrico.browser_window[windowID].domContentLoaded();
        }
    };
    let _process = null;
    let EventEmitter = require('eventemitter3');
    var process=new Proxy(new EventEmitter(), {
        get(target, prop, receiver) {
            if (prop=="stdout") {
                return {
                    write: (d) => {
                        console.log(d);
                    }
                }
            }
            if (prop=="argv") {
                let {r, e} = $e_node.syncGetStartArgs();
                return JSON.parse(r);
            }
            if (prop=="cwd") {
                return () => {
                    return window.__electrico.appPath;
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
            if (_process==null) {
                let {r, e} = $e_node.syncGetProcessInfo();
                _process = JSON.parse(r);
                for (let k in _process) {
                    target[k] = _process[k];
                }
            }
            return target[prop];
        }
    });
    window.process=process;
})();
require("./node.js");
require("./electron.js");