(function () {
    let EventEmitter = require('eventemitter3');
    window.__electrico = window.__electrico || {libs:{}};
    let uuidv4 = window.__uuidv4;
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
                    $e_electron.asyncBrowserWindowDevTools({"params":{"id":this.id, "call": "Open"}});
                }
                closeDevTools() {
                    $e_electron.asyncBrowserWindowDevTools({"params":{"id":this.id, "call": "Close"}});
                }
                executeJavaScript (script) {
                    $e_electron.asyncExecuteJavascript({"id":this.id, "script":script});
                }
                printToPDF (options) {
                    $e_electron.syncPrintToPDF({"id":this.id});
                    return "";
                }
                send (channel, ...args) {
                    $e_electron.asyncChannelSendMessage({"id":this.id, "channel":channel, "args":JSON.stringify(args)});
                }
            }
            this.webContents = new WebContentsCls(this.id);
            this.getContentBounds = (() => {
                let {r, e} = $e_electron.syncBrowserWindowBounds({"id":this.id, "params": {"method": "Get"}});
                return JSON.parse(r);
            }).bind(this);
            this.setContentBounds = ((bounds , animate) => {
                $e_electron.asyncBrowserWindowBounds({"id":this.id, "params": {"method":"Set", "bounds":bounds}});
            }).bind(this);
            this.isMaximized = (() => {
                let {r, e} = $e_electron.syncBrowserWindowMaximized({"id":this.id, "id":this.id, "params": {"method": "Get"}});
                return r=="true";
            }).bind(this);
            this.maximize = (() => {
                $e_electron.asyncBrowserWindowMaximized({"id":this.id, "params": {"method":"Set", "maximized":true}});
            }).bind(this);
            this.unmaximize = (() => {
                $e_electron.asyncBrowserWindowMaximized({"id":this.id, "params": {"method":"Set", "maximized":false}});
            }).bind(this);
            this.isMinimized = (() => {
                let {r, e} = $e_electron.syncBrowserWindowMinimized({"id":this.id, "id":this.id, "params": {"method": "Get"}});
                return r=="true";
            }).bind(this);
            this.minimize = (() => {
                $e_electron.asyncBrowserWindowMinimized({"id":this.id, "params": {"method":"Set", "minimized":true}});
            }).bind(this);
            this.close = (() => {
                window.__electrico.callAppOn("window-close", this.id);
            }).bind(this);
            this.show = (() => {
                $e_electron.asyncBrowserWindowShow({"id":this.id, "id":this.id, "shown":true});
            }).bind(this);
            this.hide = (() => {
                $e_electron.asyncBrowserWindowShow({"id":this.id, "id":this.id, "shown":false});
            }).bind(this);
            window.__electrico.browser_window[this.id]=this;
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
            let {r, e} = $e_electron.syncBrowserWindowCreate({"id":this.id, "params":{"id":this.id, "config": this.config}});
        }
        
        loadFile(file) {
            $e_electron.asyncBrowserWindowLoadfile({"params":{"id":this.id, "file":file, "config": this.config}});
        }
        loadURL(url) {
            $e_electron.asyncBrowserWindowLoadfile({"params":{"id":this.id, "file":url, "config": this.config}});
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
                },
                getSwitchValue: (k) => {
                    console.log("commandLine.getSwitchValue", k);
                    return null;
                }
            }
            this.enableSandbox = () => {
                console.log("app.enableSandbox");
            }
            this.setPath = (k, path) => {
                console.log("commandLine.setPath", k, path);
            }
            this.getPreferredSystemLanguages = () => {
                return ['en-US'];
            }
            this.getLocale = () => {
                return 'en-US';
            }
            setTimeout(()=>{
                this.emit("ready");
            }, 1000);
        }
        setName (name) {
            window.__electrico.app.name=name;
            $e_electron.asyncAppSetName({"name": name});
        }
        getName() {
            return window.__electrico.app.name;
        }
        getAppPath() {
            let {r, e} = $e_electron.syncGetAppPath();
            return r;
        }
        getPath(path) {
            let {r, e} = $e_electron.syncGetAppPath({"path":path});
            return r;
        }
        whenReady () {
            return {
                then: (cb) => {
                    cb();
                }
            };
        }
        quit() {
            $e_electron.asyncAppQuit({"exit":false});
        }
        exit() {
            $e_electron.asyncAppQuit({"exit":true});
        }
        getVersion(){
            let {r, e} = $e_electron.syncGetAppVersion();
            return r;
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
                },
                protocol: {
                    interceptFileProtocol: (schema, handler) => {
                        console.log("interceptFileProtocol", schema);
                        //TODO not implemented
                    },
                    registerFileProtocol: (schema, handler) => {
                        console.log("registerFileProtocol", schema);
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
                $e_electron.asyncSetApplicationMenu({"menu": menu});
            },
            getApplicationMenu() {
                return window.__electrico.app_menu.menu;
            }
        },
        screen: {
            getPrimaryDisplay: () => {
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
                let {r, e} = $e_electron.syncShowOpenDialogSync({options:options});
                return JSON.parse(r);
            },
            showOpenDialog: (win, options) => {
                if (options==null) {
                    options=win;
                    win=null;
                }
                return new Promise(resolve => {
                    $e_electron.asyncShowOpenDialog({"window_id": win!=null?win.id:null, options:options}).then((e, r)=>{
                        if (e!=null) {
                            let res = JSON.parse(r);
                            resolve({"canceled": res==null, "filePaths":res});
                        } else throw "showOpenDialog failed: "+e;
                    });
                });
            },
            showSaveDialogSync: (win, options) => {
                if (options==null) {
                    options=win;
                    win=null;
                }
                let {r, e} = $e_electron.syncShowSaveDialogSync({options:options});
                JSON.parse(r);
            },
            showSaveDialog: (win, options) => {
                if (options==null) {
                    options=win;
                    win=null;
                }
                return new Promise(resolve => {
                    $e_electron.asyncShowSaveDialog({"window_id": win!=null?win.id:null, options:options}).then((e, r)=>{
                        if (e!=null) {
                            let res = JSON.parse(r);
                            resolve({"canceled": res==null, "filePaths":res});
                        } else throw "showOpenDialog failed: "+e;
                    });
                });
            },
            showMessageBoxSync: (win, options) => {
                if (options==null) {
                    options=win;
                    win=null;
                }
                let {r, e} = $e_electron.syncShowMessageBoxSync({options:options});
                JSON.parse(r);
            }
        },
        shell: {
            openExternal: (url, options) => {
                $e_electron.asyncShellOpenExternal({url:url});
            },
            openPath: (path, options) => {
                $e_electron.asyncShellOpenExternal({url:path});
            }
        },
        protocol: {
            registerSchemesAsPrivileged: (customSchemes) => {
                console.log("registerSchemesAsPrivileged", customSchemes);
            }
        },
        crashReporter: {

        },
        contentTracing: {

        }
    };

    window.__electrico.libs["electron/main"]=electron;
    window.__electrico.libs["electron"]=electron;

    var {Buffer} = require("buffer");
    window.Buffer=Buffer;

    let {r, e} = $e_electron.syncGetAppPath();
    window.__electrico.appPath = r;
})();