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
    window.__init_shared(window, true);
    function createLogMsg(level, logmsg, logdata) {
        return {"action":"Node", invoke:{command:"ConsoleLog", "params":{"level": level, "logmsg":logmsg, "logdata":JSON.stringify(logdata)}}};
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
        const req = createCMDRequest(true);
        req.send(JSON.stringify(createLogMsg("Info", logmsg+"", logdata)));
    };
    window.console.info = window.console.log;
    window.console.debug = (logmsg, ...logdata) => {
        console_debug(logmsg, ...logdata);
        const req = createCMDRequest(true);
        req.send(JSON.stringify(createLogMsg("Debug", logmsg+"", logdata)));
    };
    window.console.error = (logmsg, ...logdata) => {
        console_error(logmsg, ...logdata);
        const req = createCMDRequest(true);
        for (let i=0; i<logdata.length; i++) {
            if (logdata[i] instanceof Error) logdata[i]=logdata[i].message;
        }
        req.send(JSON.stringify(createLogMsg("Error", logmsg+"", logdata)));
    };
    window.console.warn = (logmsg, ...logdata) => {
        console_warn(logmsg, ...logdata);
        const req = createCMDRequest(true);
        req.send(JSON.stringify(createLogMsg("Warn", logmsg+"", logdata)));
    };
    window.console.trace = (logmsg, ...logdata) => {
        console_trace(logmsg, ...logdata);
        const req = createCMDRequest(true);
        req.send(JSON.stringify(createLogMsg("Trace", logmsg+"", logdata)));
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
                const req = createCMDRequest(true);
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
                on_stdout: (pid, data) => {
                    let cb = window.__electrico.child_process[pid].stdout_on['data'];
                    if (cb!=null) {
                        cb(data);
                    }
                },
                on_stderr: (pid, data) => {
                    let cb = window.__electrico.child_process[pid].stderr_on['data'];
                    if (cb!=null) {
                        cb(data);
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
            window.__import_meta = {url:""};
            window.__dirname = window.__electrico.appPath+(main.indexOf("/")>=0?("/"+main.substring(0, main.indexOf("/"))):"");
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
                            const req = createCMDRequest(true);
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
                        const req = createCMDRequest(true);
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
    
    var process=new Proxy({}, {
        get(target, prop, receiver) {
            if (prop=="stdout") {
                return {
                    write: (d) => {
                        const req = createCMDRequest(true);
                        req.send(JSON.stringify(createLogMsg("Info", d)));
                    }
                }
            }
            if (prop=="argv") {
                const req = createCMDRequest(false);
                req.send(JSON.stringify({"action":"Node", invoke:{command:"GetStartArgs"}}));
                return JSON.parse(req.responseText);
            }
            if (prop=="on") {
                return (event, f) => {
                    //console.log("process on", event, f);
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
                const req = createCMDRequest(false);
                req.send(JSON.stringify({"action":"Node", invoke:{command:"GetProcessInfo"}}));
                _process = JSON.parse(req.responseText);
            }
            return _process[prop];
        }
    });
    window.process=process;
})();