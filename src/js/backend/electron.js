(function () {
    let EventEmitter = require('eventemitter3');
    window.__electrico = window.__electrico || {libs:{}};
    function uuidv4() {
        return "10000000-1000-4000-8000-100000000000".replace(/[018]/g, c =>
            (+c ^ window.crypto.getRandomValues(new Uint8Array(1))[0] & 15 >> +c / 4).toString(16)
        );
    }
    function wrapInvoke(invoke) {
        return {"action":"Electron", invoke:invoke};
    }
    window.__electrico.wrapInvoke=wrapInvoke;
    function menuBuildFromTemplate(menu, idmapping) {
        let _idmapping = idmapping || {};
        for (let sub of menu) {
            sub.id = uuidv4();
            _idmapping[sub.id] = sub;
            if (sub.submenu!=null) {
                menuBuildFromTemplate(sub.submenu, _idmapping);
            }
        }
        return _idmapping;
    }
    
    class BrowserWindow extends EventEmitter {
        constructor(config) {
            super();
            if (config.x != null) config.x=Math.floor(config.x);
            if (config.y != null) config.y=Math.floor(config.y);
            this.id="browser_window_"+uuidv4();
            this.config=config;
            class WebContentsCls extends EventEmitter {
                constructor(id) {
                    super();
                    this.id=id;
                }
                openDevTools() {
                    const req = createCMDRequest(true);
                    req.send(JSON.stringify(wrapInvoke({"command":"BrowserWindowDevTools", "params":{"id":this.id, "call": "Open"}}))); 
                }
                closeDevTools() {
                    const req = createCMDRequest(true);
                    req.send(JSON.stringify(wrapInvoke({"command":"BrowserWindowDevTools", "params":{"id":this.id, "call": "Close"}}))); 
                }
                executeJavaScript (script) {
                    const req = createCMDRequest(true);
                    req.send(JSON.stringify(wrapInvoke({"command":"ExecuteJavascript", "id":this.id, "script":script}))); 
                }
                printToPDF (options) {
                    const req = createCMDRequest(false);
                    req.send(JSON.stringify(wrapInvoke({"command":"PrintToPDF", "id":this.id})));
                    return "";
                }
                send (channel, ...args) {
                    const req = createCMDRequest(true);
                    req.send(JSON.stringify(wrapInvoke({"command":"ChannelSendMessage", "id":this.id, "channel":channel, "args":JSON.stringify(args)}))); 
                }
            }
            this.webContents = new WebContentsCls(this.id);
            this.getContentBounds = (() => {
                const req = createCMDRequest(false);
                req.send(JSON.stringify(wrapInvoke({"command":"BrowserWindowBounds", "id":this.id, "params": {"method": "Get"}})));
                return JSON.parse(req.responseText);
            }).bind(this);
            this.setContentBounds = ((bounds , animate) => {
                const req = createCMDRequest(true);
                req.send(JSON.stringify(wrapInvoke({"command":"BrowserWindowBounds", "id":this.id, "params": {"method":"Set", "bounds":bounds}})));
            }).bind(this);
            this.isMaximized = (() => {
                const req = createCMDRequest(false);
                req.send(JSON.stringify(wrapInvoke({"command":"BrowserWindowMaximized", "id":this.id, "params": {"method": "Get"}})));
                return req.responseText=="true";
            }).bind(this);
            this.maximize = (() => {
                const req = createCMDRequest(true);
                req.send(JSON.stringify(wrapInvoke({"command":"BrowserWindowMaximized", "id":this.id, "params": {"method":"Set", "maximized":true}})));
            }).bind(this);
            this.unmaximize = (() => {
                const req = createCMDRequest(true);
                req.send(JSON.stringify(wrapInvoke({"command":"BrowserWindowMaximized", "id":this.id, "params": {"method":"Set", "maximized":false}})));
            }).bind(this);
            this.isMinimized = (() => {
                const req = createCMDRequest(false);
                req.send(JSON.stringify(wrapInvoke({"command":"BrowserWindowMinimized", "id":this.id, "params": {"method": "Get"}})));
                return req.responseText=="true";
            }).bind(this);
            this.minimize = (() => {
                const req = createCMDRequest(true);
                req.send(JSON.stringify(wrapInvoke({"command":"BrowserWindowMinimized", "id":this.id, "params": {"method":"Set", "minimized":true}})));
            }).bind(this);
            this.close = (() => {
                window.__electrico.callAppOn("window-close", this.id);
            }).bind(this);
            this.show = (() => {
                const req = createCMDRequest(true);
                req.send(JSON.stringify(wrapInvoke({"command":"BrowserWindowShow", "id":this.id, "shown":true}))); 
            }).bind(this);
            this.hide = (() => {
                const req = createCMDRequest(true);
                req.send(JSON.stringify(wrapInvoke({"command":"BrowserWindowShow", "id":this.id, "shown":false}))); 
            }).bind(this);
            window.__electrico.browser_window[this.id]=this;
            const req = createCMDRequest(false);
            this.config.title = this.config.title || "Electrico Window";
            this.config.resizable = this.config.resizable!=null?this.config.resizable:true;
            this.config.modal = this.config.modal!=null?this.config.modal:false;
            this.config.show = this.config.show!=null?this.config.show:true;
            if (this.config.webPreferences==null) {
                this.config.webPreferences={};
            }
            if (this.config.webPreferences.nodeIntegration==null) {
                this.config.webPreferences.nodeIntegration=false;
            }
            if (this.config.webPreferences.contextIsolation==null) {
                this.config.webPreferences.contextIsolation=true;
            }
            req.send(JSON.stringify(wrapInvoke({"command":"BrowserWindowCreate", "params":{"id":this.id, "config": this.config}}))); 
        }
        
        loadFile(file) {
            const req = createCMDRequest(true);
            req.send(JSON.stringify(wrapInvoke({"command":"BrowserWindowLoadfile", "params":{"id":this.id, "file":file, "config": this.config}}))); 
        }
        loadURL(url) {
            const req = createCMDRequest(true);
            req.send(JSON.stringify(wrapInvoke({"command":"BrowserWindowLoadfile", "params":{"id":this.id, "file":url, "config": this.config}}))); 
        }
        removeMenu = () => {
            console.log("BrowserWindow.removeMenu");
        }
        domContentLoaded = () => {
            this.webContents.emit("did-finish-load");
            this.webContents.emit("dom-ready");
        }
    };
    BrowserWindow.getAllWindows = () => {
        let windows = [];
        for (let id in window.__electrico.browser_window) {
            windows.push(window.__electrico.browser_window[id]);
        }
        return windows;
    };
    class AppCls extends EventEmitter {
        constructor() {
            super();
            this.commandLine = {
                appendSwitch: (...args) => {
                    console.log("commandLine.appendSwitch", args);
                }
            }
        }
        setName (name) {
            window.__electrico.app.name=name;
            const req = createCMDRequest(true);
            req.send(JSON.stringify(wrapInvoke({"command":"AppSetName", "name": name})));
        }
        getName() {
            return window.__electrico.app.name;
        }
        getAppPath() {
            const req = createCMDRequest(false);
            req.send(JSON.stringify(wrapInvoke({"command":"GetAppPath"})));
            return req.responseText;
        }
        getPath(path) {
            const req = createCMDRequest(false);
            req.send(JSON.stringify(wrapInvoke({"command":"GetAppPath", "path":path})));
            return req.responseText;
        }
        whenReady () {
            return {
                then: (cb) => {
                    cb();
                }
            };
        }
        quit() {
            const req = createCMDRequest(true);
            req.send(JSON.stringify(wrapInvoke({"command":"AppQuit", "exit":false})));
        }
        exit() {
            const req = createCMDRequest(true);
            req.send(JSON.stringify(wrapInvoke({"command":"AppQuit", "exit":true})));
        }
        getVersion(){
            const req = createCMDRequest(false);
            req.send(JSON.stringify(wrapInvoke({"command":"GetAppVersion"})));
        }
        requestSingleInstanceLock(ad) {
            return true;
        }
    }
    let electron = {
        session: {
            defaultSession: {
                webRequest: {
                    onHeadersReceived: (handler) => {
                        //TODO not implemented
                    }
                }
            }
        },
        app: new AppCls(),
        ipcMain: {
            on: (channel, fun) => {
                window.__electrico.channel[channel]=fun;
            },
            handle: (channel, fun) => {
                window.__electrico.channel[channel]=fun;
            },
            off: (channel, fun) => {
                delete window.__electrico.channel[channel];
            },
        },
        BrowserWindow: BrowserWindow,
        Menu: {
            buildFromTemplate(template) {
                let idmapping = menuBuildFromTemplate(template);
                window.__electrico.app_menu.idmapping=idmapping;
                return template;
            },
            setApplicationMenu(menu) {
                window.__electrico.app_menu.menu=menu;
                const req = createCMDRequest(true);
                req.send(JSON.stringify(wrapInvoke({"command":"SetApplicationMenu", "menu": menu})));
            },
            getApplicationMenu() {
                return window.__electrico.app_menu.menu;
            }
        },
        screen: {
            getPrimaryDisplay: () => {
                /*const req = createCMDRequest(false);
                req.send(JSON.stringify(wrapInvoke({"command":"GetPrimaryDisplay"})));
                let res = JSON.parse(req.responseText);
                return res!=null?JSON.parse(req.responseText):undefined;*/
                return {
                    bounds: {
                        width:window.screen.width,
                        height:window.screen.height,
                        x:0,
                        y:0
                    }
                };
            },
            getAllDisplays: () => {
                return [{
                    bounds: {
                        width:window.screen.width,
                        height:window.screen.height,
                        x:0,
                        y:0
                    }
                }];
            }
        },
        dialog: {
            showOpenDialogSync: (win, options) => {
                if (options==null) {
                    options=win;
                    win=null;
                }
                const req = createCMDRequest(false);
                req.send(JSON.stringify(wrapInvoke({"command":"ShowOpenDialogSync", options:options})));
                let res = JSON.parse(req.responseText);
                return res!=null?JSON.parse(req.responseText):undefined;
            },
            showOpenDialog: (win, options) => {
                if (options==null) {
                    options=win;
                    win=null;
                }
                return new Promise(resolve => {
                    const req = createCMDRequest(true);
                    req.onreadystatechange = function() {
                        if (this.readyState == 4) {
                            if (req.status == 200) {
                                let res = JSON.parse(req.responseText);
                                resolve({"canceled": res==null, "filePaths":res});
                            } else throw "showOpenDialog failed: "+req.status;
                        }
                    };
                    req.send(JSON.stringify(wrapInvoke({"command":"ShowOpenDialog", "window_id": win!=null?win.id:null, options:options})));
                });
            },
            showSaveDialogSync: (win, options) => {
                if (options==null) {
                    options=win;
                    win=null;
                }
                const req = createCMDRequest(false);
                req.send(JSON.stringify(wrapInvoke({"command":"ShowSaveDialogSync", options:options})));
                let res = JSON.parse(req.responseText);
                return res!=""?req.responseText:undefined;
            },
            showSaveDialog: (win, options) => {
                if (options==null) {
                    options=win;
                    win=null;
                }
                return new Promise(resolve => {
                    const req = createCMDRequest(true);
                    req.onreadystatechange = function() {
                        if (this.readyState == 4) {
                            if (req.status == 200) {
                                let res = req.responseText;
                                console.log("showSaveDialog response", res);
                                resolve({"canceled": res=="", "filePath":res});
                            } else throw "showSaveDialog failed: "+req.status;
                        }
                    };
                    req.send(JSON.stringify(wrapInvoke({"command":"ShowSaveDialog", "window_id": win!=null?win.id:null, options:options})));
                });
            },
            showMessageBoxSync: (win, options) => {
                if (options==null) {
                    options=win;
                    win=null;
                }
                const req = createCMDRequest(false);
                req.send(JSON.stringify(wrapInvoke({"command":"ShowMessageBoxSync", options:options})));
                let res = JSON.parse(req.responseText);
                return res!=null?JSON.parse(req.responseText):undefined;
            }
        },
        shell: {
            openExternal: (url, options) => {
                const req = createCMDRequest(true);
                req.send(JSON.stringify(wrapInvoke({"command":"ShellOpenExternal", url:url})));
            },
            openPath: (path, options) => {
                const req = createCMDRequest(true);
                req.send(JSON.stringify(wrapInvoke({"command":"ShellOpenExternal", url:path})));
            }
        }
    };

    window.__electrico.libs["electron/main"]=electron;
    window.__electrico.libs["electron"]=electron;

    var {Buffer} = require("buffer");
    window.Buffer=Buffer;

    const req = createCMDRequest(false);
    req.send(JSON.stringify(wrapInvoke({"command":"GetAppPath"})));
    window.__electrico.appPath = req.responseText;
})();