(function () {
    const EventEmitter = require('eventemitter3');
    window.__electrico = window.__electrico || {libs:{}};
    let uuidv4 = window.__uuidv4;
    
    class MenuItem extends EventEmitter {
        constructor(options) {
            super();
            this.items = [];
            for (let k in options) {
                this[k] = options[k];
            }
        }
    }
    class Menu extends EventEmitter {
        constructor() {
            super();
            this.items = [];
            this.append = (mi) => {
                this.items.push(mi);
            }
        }
    }
    function menuBuildFromTemplate(tpl) {
        let menu = new Menu();
        for (let sub of tpl) {
            let mi = new MenuItem(sub);
            menu.append(mi);
            if (sub.submenu!=null) {
                mi.submenu = menuBuildFromTemplate(sub.submenu);
            }
        }
        return menu; 
    }
    Menu.buildFromTemplate = (template) => {
        let menu = menuBuildFromTemplate(template);
        return menu;
    };
    function menubuildWithIds(menu, idmapping) {
        let emenu = [];
        let _idmapping = idmapping || {};
        for (let mi of menu.items) {
            mi.id = uuidv4();
            _idmapping[mi.id] = mi;
            let emi = {...mi};
            emenu.push(emi);
            if (mi.submenu!=null) {
                let {menu} = menubuildWithIds(mi.submenu, _idmapping);
                emi.submenu = menu;
            }
        }
        return {menu:emenu, idmapping:_idmapping};
    }

    Menu.setApplicationMenu = (_menu) => {
        window.__electrico.app_menu.menu=_menu;
        if (_menu!=null) {
            let {menu, idmapping} = menubuildWithIds(_menu);
            window.__electrico.app_menu.idmapping=idmapping;
            $e_electron.asyncSetApplicationMenu({"menu": menu});
        } else {
            window.__electrico.app_menu.idmapping=null;
            $e_electron.asyncSetApplicationMenu({"menu": null});
        }
    }
    Menu.getApplicationMenu = () => {
        return window.__electrico.app_menu.menu;
    }
    let _wid=0;
    class BrowserWindow extends EventEmitter {
        constructor(config) {
            super();
            if (config.x != null) config.x=Math.floor(config.x);
            if (config.y != null) config.y=Math.floor(config.y);
            this.id=_wid++;
            this._e_id="browser_window_"+uuidv4();
            this.config=config;
            class WebContentsCls extends window.__electrico.ProcessPort {
                constructor(win, _e_id) {
                    super((msg) => {
                        let channel = msg.portid;
                        if (channel==null) {
                            channel = msg.data.channel;
                            msg.data = msg.data.message;
                            msg.fromWebContents = true;
                        }
                        let rid = uuidv4();
                        let data_blob=null;
                        if (msg.data._electrico_args==null) {
                            msg.posted=true;
                            let Buffer = require('buffer').Buffer;
                            if (Buffer.isBuffer(msg.data)) {
                                data_blob=msg.data;
                                msg.data={_electrico_buffer_id:rid};
                            }
                            let ipcMain = require("electron").ipcMain;
                            for (let p of msg.ports) {
                                ipcMain.on(p.id, (function(event, message) {
                                    this.onMessageReceived({data:message, portid:p.id, ports:[]});
                                }).bind(this));
                            }
                        } else {
                            msg = msg.data._electrico_args;
                        }
                        let action = {"action":"Electron", "invoke":{"command":"ChannelSendMessage", "id":this._e_id, "channel":channel, "args":JSON.stringify(msg)}};
                        let action_msg = {"command": action, "data_blob":data_blob!=null};
                        window.__ipc_websocket("ipc", false, null, null, (socket)=>{
                            let msg = (new TextEncoder()).encode(JSON.stringify(action_msg));
                            socket.send(msg);
                            if (data_blob!=null) {
                                socket.send(data_blob);
                            }
                        });
                    }, {
                        deserialize:(msg, cb) => {cb(msg)},
                        serialize:(msg, cb) => {cb(msg)}
                    });
                    this._e_id=_e_id;
                    this.isDestroyed = (()=>{
                        return false;
                    }).bind(this);
                    this.session = {
                        webRequest: {
                            onBeforeSendHeaders: (...args) => {
                                //TODO
                                console.log("session.webRequest.onBeforeSendHeaders", args);
                            }
                        }
                    };
                    this.setWindowOpenHandler = (h) => {
                        console.log("setWindowOpenHandler", h);
                    };
                    this.getOwnerBrowserWindow = () => {
                        return win;
                    };
                    let _postMessage = this.postMessage;
                    this.postMessage = (channel, message, ports) => {
                        if (ports!=null && ports.length>0) {
                            ports.map((p) => {
                                if (p.connected_port!=null) {
                                    if (p.connected_port._posted_remote!=null) {
                                        let cmsg = {id:p.id, remote_id:p.connected_port.id, clientid:p.connected_port._posted_remote.clientid, hook:p.connected_port._posted_remote.hook};
                                        _postMessage({channel:"__posted_remote_connect_hook", message:cmsg});
                                        delete p.connected_port._posted_remote;
                                    } else {
                                        p._posted_renderer = {
                                            connect_hook: function(clientid, hook) {
                                                console.log("connect_hook");
                                                let cmsg = {id:p.id, remote_id:p.connected_port.id, clientid:clientid, hook:hook};
                                                _postMessage({channel:"__posted_remote_connect_hook", message:cmsg});
                                                delete p._posted_renderer.connect_hook;
                                            }
                                        };
                                    }
                                }
                            });
                        }
                        _postMessage({channel:channel, message:message}, ports);
                    };
                    this.getOSProcessId = ()=> {
                        return process.pid;
                    };
                    this.getProcessId = ()=> {
                        return process.pid;
                    };
                    this.setIgnoreMenuShortcuts = (ignore) => {
                        //TODO
                    };
                    this.send = (function(channel, ...args) {
                        this.postMessage(channel, args.length==1?args[0]:{_electrico_args:args});
                    }).bind(this);
                }
                openDevTools() {
                    $e_electron.asyncBrowserWindowDevTools({"params":{"id":this._e_id, "call": "Open"}});
                }
                closeDevTools() {
                    $e_electron.asyncBrowserWindowDevTools({"params":{"id":this._e_id, "call": "Close"}});
                }
                executeJavaScript (script) {
                    $e_electron.asyncExecuteJavascript({"id":this._e_id, "script":script});
                }
                printToPDF (options) {
                    $e_electron.syncPrintToPDF({"id":this._e_id});
                    return "";
                }
                getLastWebPreferences() {
                    return {
                        enableRemoteModule: true
                    }
                }
            }
            this.webContents = new WebContentsCls(this, this._e_id);
            this.webContents.start();
            setTimeout((()=>{
                electron.app.emit("web-contents-created", "web-contents-created", this.webContents)
            }).bind(this), 1);
            this.getContentBounds = (() => {
                let {r, e} = $e_electron.syncBrowserWindowBounds({"id":this._e_id, "params": {"method": "Get"}});
                return JSON.parse(r);
            }).bind(this);
            this.setContentBounds = ((bounds , animate) => {
                $e_electron.asyncBrowserWindowBounds({"id":this._e_id, "params": {"method":"Set", "bounds":bounds}});
            }).bind(this);
            this.isMaximized = (() => {
                let {r, e} = $e_electron.syncBrowserWindowMaximized({"id":this._e_id, "params": {"method": "Get"}});
                return r=="true";
            }).bind(this);
            this.maximize = (() => {
                $e_electron.asyncBrowserWindowMaximized({"id":this._e_id, "params": {"method":"Set", "maximized":true}});
            }).bind(this);
            this.unmaximize = (() => {
                $e_electron.asyncBrowserWindowMaximized({"id":this._e_id, "params": {"method":"Set", "maximized":false}});
            }).bind(this);
            this.isMinimized = (() => {
                let {r, e} = $e_electron.syncBrowserWindowMinimized({"id":this._e_id, "params": {"method": "Get"}});
                return r=="true";
            }).bind(this);
            this.minimize = (() => {
                $e_electron.asyncBrowserWindowMinimized({"id":this._e_id, "params": {"method":"Set", "minimized":true}});
            }).bind(this);
            this.setFullScreen = (fs => {
                this.maximize();
            }).bind(this);
            this.isFullScreen = (() => {
                return this.isMaximized();
            }).bind(this);
            this.setSimpleFullScreen = (fs => {
                this.maximize();
            }).bind(this);
            this.isSimpleFullScreen = (() => {
                return this.isMaximized();
            }).bind(this);
            this.setPosition = ((x, y, animate) => {
                let b = this.getContentBounds();
                this.setContentBounds({x:x, y:y, width:b.width, height:b.height});
            }).bind(this);
            this.getPosition = (() => {
                let b = this.getContentBounds();
                return [b.x, b.y];
            }).bind(this);
            this.setSize = ((width, height, animate) => {
                let b = this.getContentBounds();
                this.setContentBounds({x:b.x, y:b.y, width:width, height:height});
            }).bind(this);
            this.getSize = (() => {
                let b = this.getContentBounds();
                return [b.width, b.height];
            }).bind(this);
            this.getMinimumSize = (() => {
                return this.getSize();
            }).bind(this);
            this.setMinimumSize = ((width, height) => {
                //this.setSize(width, height);
            }).bind(this);
            this.close = (() => {
                console.log("BrowserWindow.close");
                window.__electrico.callAppOn("window-close", this._e_id);
            }).bind(this);
            this.show = (() => {
                $e_electron.asyncBrowserWindowShow({"id":this._e_id, "shown":true});
            }).bind(this);
            this.hide = (() => {
                $e_electron.asyncBrowserWindowShow({"id":this._e_id, "shown":false});
            }).bind(this);
            this.setTitle = ((title) => {
                $e_electron.asyncBrowserWindowSetTitle({"id":this._e_id, "title":title});
            }).bind(this);
            this.getTitle = (() => {
                let {e, r} = $e_electron.syncBrowserWindowGetTitle({"id":this._e_id});
                return r;
            }).bind(this);
            this.isDestroyed = (()=>{
                return this._destroyed;
            }).bind(this);
            this.destroy = (()=>{
                this._destroyed=true;
                this.close();
            }).bind(this);
            this.setSheetOffset = ((ox, oy) => {
                //TODO
            }).bind(this);
            this.setWindowButtonPosition = (p => {
                //TODO
            }).bind(this);
            this.setTouchBar = (b => {
                //TODO
            }).bind(this);
            this.setBackgroundColor = (c => {
                //TODO
            }).bind(this);
            this.setDocumentEdited = ((ed) => {
                this._documentEdited = ed;
            }).bind(this);
            this.isDocumentEdited = (() => {
                return this._ed;
            }).bind(this);
            this.setRepresentedFilename = ((fn) => {
                this._representedFilename = fn;
            }).bind(this);
            this.getRepresentedFilename = (() => {
                return this._representedFilename;
            }).bind(this);
            this._focused=true;
            this.focus = (() => {
                // TODO focus window
                this._focused=true;
            }).bind(this);
            this.blur = (() => {
                this._focused=false;
            }).bind(this);
            this.isFocused = (() => {
                return this._focused;
            }).bind(this);
            this.isVisible = (() => {
                return true;
            }).bind(this);
            this.getNativeWindowHandle = (() => {
                return Buffer.from(this._e_id);
            }).bind(this);
            
            window.__electrico.browser_window[this._e_id]=this;
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
            let {r, e} = $e_electron.syncBrowserWindowCreate({"id":this._e_id, "params":{"id":this._e_id, "config": this.config}});
        }
        
        loadFile(file) {
            $e_electron.asyncBrowserWindowLoadfile({"params":{"id":this._e_id, "file":file, "config": this.config}});
        }
        loadURL(url) {
            $e_electron.asyncBrowserWindowLoadfile({"params":{"id":this._e_id, "file":url, "config": this.config}});
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
    BrowserWindow.getFocusedWindow = () => {
        for (let id in window.__electrico.browser_window) {
            if (window.__electrico.browser_window[id].isFocused()) { //TODO check focus
               return window.__electrico.browser_window[id];
            }
        }
        return null;
    };
    BrowserWindow.fromWebContents = (contents) => {
        return contents.getOwnerBrowserWindow();
    }
    
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
            this.addRecentDocument = (path) => {
                //TODO
                console.log("app.addRecentDocument", path);
            }
            this.clearRecentDocuments = () => {
                //TODO
            };
            this.dock = {
                setMenu: (menu) => {
                    console.log("dock.setMenu", menu);
                }
            };
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
    class IPCMainCls extends EventEmitter {
        constructor() {
            super();
            let _this = this;
            this.handle = (channel, fun) => {
                _this.on(channel, fun);
            }
            this.__callIpc = function(channel, ...args) {
                let listeners = _this.listeners(channel);
                if (listeners.length==0) {
                    console.error("IPCMainCls.__callIpc no handler for channel:", channel);
                    return null;
                }
                return listeners[0](...args);
            }
        }
    }
    let electron = {
        session: {
            defaultSession: {
                webRequest: {
                    onHeadersReceived: (handler) => {
                        console.log("onHeadersReceived");
                        //TODO not implemented
                    },
                    onBeforeRequest: () => {
                        console.log("onBeforeRequest");
                        //TODO not implemented
                    }
                },
                protocol: {
                    interceptFileProtocol: (schema, handler) => {
                        console.log("interceptFileProtocol", schema);
                        //TODO not implemented
                    },
                    registerFileProtocol: (schema, handler) => {
                        console.log("session.protocol.registerFileProtocol", schema);
                        $e_electron.asyncRegisterFileProtocol({schema:schema});
                        window.__electrico.file_protocol[schema] = (requestID, request) => {
                            //console.log("file_protocol call", requestID, request);
                            request.url=schema+"://"+request.url;
                            handler(request, (response) => {
                                //console.log("file_protocol call handler response", request, response);
                                let urlcmd = JSON.stringify({"action":"SetIPCResponse", "request_id":requestID, "params": response.data, file_path:response.path});
                                const req = new XMLHttpRequest();
                                req.open("POST", window.__create_protocol_url("cmd://cmd/Frontend.SetProtocolResponse"), true);
                                req.send(urlcmd);
                            });
                        }
                    }
                },
                setPermissionRequestHandler: (handler) => {
                    console.log("setPermissionRequestHandler");
                    //TODO not implemented
                },
                setPermissionCheckHandler: (handler) => {
                    console.log("setPermissionCheckHandler");
                    //TODO not implemented
                }
            }
        },
        app: new AppCls(),
        clipboard: {
            has: (format, type)=>{return false}
        },
        ipcMain: new IPCMainCls(),
        BrowserWindow: BrowserWindow,
        Menu: Menu,
        MenuItem: MenuItem,
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
            },
            registerFileProtocol: (schema, handler) => {
                electron.session.defaultSession.protocol.registerFileProtocol(schema, handler);
            },
            registerBufferProtocol: (schema, handler) => {
                console.log("registerBufferProtocol", schema);
            },
            registerHttpProtocol: (schema, handler) => {
                console.log("registerHttpProtocol", schema);
                //TODO not implemented
            }
        },
        crashReporter: {
            start: (options) => {
                console.log("crashReporter.start", options);
            }
        },
        contentTracing: {

        },
        autoUpdater: new EventEmitter(),
        utilityProcess: {
            fork: (modulePath, args, options) => {
                if (args==null) {
                    args=options;
                    options=null;
                }
                args = args || [];
                class UtilityProcessCls extends window.__electrico.ProcessPort {
                    constructor() {
                        super();
                        this.pending_ports=[];
                        this.clientid = uuidv4();
                        console.log("this.clientid", this.clientid);
                        let _this = this;
                        this.sender = (function(data) {
                            window.__call_queue("UP"+this.clientid, (d)=>{
                                if (_this._forked) {
                                    let action = {"action":"PostIPC", "http_id":"fork", "from_backend":true, "request_id":"fork", "channel":_this.clientid, "params":"["+d.msg+"]"};
                                    let action_msg = {"command": action, "data_blob":d.data_blob!=null};
                                    window.__ipc_websocket(_this.clientid, false, null, this._fork_hook, (socket)=>{
                                        let msg = (new TextEncoder()).encode(JSON.stringify(action_msg));
                                        socket.send(msg);
                                        if (d.data_blob!=null) {
                                            socket.send(d.data_blob);
                                        }
                                    });
                                }
                                return _this._forked;
                            }, data);
                        }).bind(this);
                        this.flatten = (msg) => {
                            return msg.data;
                        }
                        this.sbuffer = new window.__electrico.SerializationBuffer(this.clientid);
                        this.forked = (function(hook) {
                            console.log("forked", hook);
                            this._forked=true;
                            this._fork_hook = hook;
                            for (let p of this.pending_ports) {
                                delete p.pending;
                            }
                            delete this.pending_ports;
                            const ipcMain = require("electron").ipcMain;
                            ipcMain.on(this.clientid, (function(e, msg) {
                                //console.log("server msg received", msg);
                                this.onMessageReceived(msg);
                            }).bind(this));
                            this.emit("spawn");
                        }).bind(this);
                        let _postMessage = this.postMessage;
                        this.postMessage = (data, ports, ...args) => {
                            if (ports!=null && ports.length>0) {
                                ports.map((p) => {
                                    if (p.connected_port!=null) {
                                        let connect_hook = function() {
                                            if (_this._forked) {
                                                if (p.connected_port._posted_renderer!=null) {
                                                    p.connected_port._posted_renderer.connect_hook(_this.clientid, _this._fork_hook);
                                                } else {
                                                    p._posted_remote={clientid:_this.clientid, hook:_this._fork_hook}
                                                }
                                            } else {
                                                setTimeout(connect_hook, 100);
                                            }
                                        };
                                        connect_hook();
                                    }
                                });
                            }
                            _postMessage(data, ports, ...args);      
                        };
                    }
                }
                uProc = new UtilityProcessCls();
                uProc.start();
                
                const { spawn } = require('node:child_process');
                let ix = modulePath.indexOf("/")+1;
                if (modulePath.startsWith(window.__electrico.appPath)) {
                    ix = window.__electrico.appPath.length;
                    if (!window.__electrico.appPath.endsWith("/")) ix+=1;
                }
                let moduleSrc = modulePath.substring(0, ix);
                let moduleMain = modulePath.substring(ix);
                if (moduleMain.startsWith("/")) moduleMain = moduleMain.substring(1);
                options = options || {};
                if (options.env==null) options.env = process.env;
                let main_hook = "ws://electrico.localhost:"+window.__http_protocol.http_port+"/"+window.__http_protocol.http_uid+"@asyncin/parent_"+process.pid;
                let fork = {args:args, ...options, moduleSrc:moduleSrc, moduleMain:moduleMain, hook:main_hook, clientid:uProc.clientid};
                fork.env = JSON.stringify(fork.env);
                let e_args = ["-f", JSON.stringify(fork)];
                if (options.execArgv!=null) {
                    for (let a of options.execArgv) {
                        //e_args.push(a);
                    }
                }
                let child = spawn(process.execPath, e_args);
                uProc.stdout = child.stdout;
                uProc.stderr = child.stderr;
                /*child.stderr.on("data", (msg)=>{
                    console.log("Fork STDERR:"+(new TextDecoder()).decode(msg));
                });
                child.stdout.on("data", (msg)=>{
                    console.log("Fork STDOUT"+(new TextDecoder()).decode(msg));
                });*/
                uProc.pid = child.pid;
                let _child_emit = child.emit;
                child.emit = function(...args) {
                    _child_emit.bind(child)(...args);
                    uProc.emit(...args);
                }
                uProc.kill = child.kill;
                const ipcMain = require("electron").ipcMain;
                (function(proc) {
                    const initfork = function(e, msg) {
                        console.log("fork initialized", proc.clientid, msg);
                        ipcMain.removeListener(proc.clientid, initfork);
                        proc.forked(msg.data.hook);
                    };
                    ipcMain.on(proc.clientid, initfork);
                })(uProc);
                console.log("process forked!!!!");
                return uProc;
            }
        },
        MessageChannelMain: class {
            constructor() {
                class ChannelPort extends EventEmitter {
                    constructor(port2) {
                        super();
                        this.id = uuidv4();
                        this.connected_port = port2;
                        if (port2!=null) {
                            port2.connected_port=this;
                        }
                        this.postMessage = ((data, ports) => {
                            if (this.started) {
                                let _this = this;
                                window.__call_queue("MC"+this.id, (msg)=>{
                                    if (_this.connected_port.send_locked) {
                                        console.log("send_locked!!");
                                        return false;
                                    } else {
                                        _this.connected_port.emit("message", msg);
                                        return true;
                                    }
                                }, {data, ports});
                            } else {
                                console.error("postMessage ChannelPort not started", this.id);
                            }
                        }).bind(this);
                        this.start = (() => {
                            this.started=true;
                        }).bind(this);
                        this.close = (() => {
                            this.started=false;
                        }).bind(this);
                    }
                }
                this.port1 = new ChannelPort();
                this.port1.start();
                this.port2 = new ChannelPort(this.port1);
                this.port2.start();
            }
        }
    };
    electron.nativeTheme=new EventEmitter();
    electron.powerMonitor=new EventEmitter();
    electron.screen=new EventEmitter();
    electron.screen.getPrimaryDisplay = () => {
        return {
            bounds: {
                width:window.screen.width,
                height:window.screen.height,
                x:0,
                y:0
            },
            workArea: {
                width:window.screen.width,
                height:window.screen.height,
                x:0,
                y:0
            }
        };
    };
    electron.screen.getAllDisplays = () => {
        return [{
            bounds: {
                width:window.screen.width,
                height:window.screen.height,
                x:0,
                y:0
            },
            workArea: {
                width:window.screen.width,
                height:window.screen.height,
                x:0,
                y:0
            }
        }];
    };
    electron.TouchBar=class {};
    electron.TouchBar.TouchBarSegmentedControl = class {};
    
    electron.main = electron;
    window.__electrico.libs["electron"]=electron;

    var {Buffer} = require("buffer");
    window.Buffer=Buffer;

    let {r, e} = $e_electron.syncGetAppPath();
    window.__electrico.appPath = r;

    require("./apis/apis.js");
    delete window.indexedDB;
})();