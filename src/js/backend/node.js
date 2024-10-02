(function () {
    //let path = require('path');
    let uuidv4 = window.__uuidv4;
    var global = window;
    window.global = global;
    let Buffer = require('buffer').Buffer;
    let EventEmitter = require('eventemitter3');
    let {queryString} = require('query-string');
    window.__electrico.libs.util = {};
    let inherits = require('inherits');
    window.__electrico.libs["node:inherits"] = inherits;
    window.__electrico.libs.inherits = inherits;
    window.__electrico.libs.util = null;
    let util = require('util');
    let _fd=0;
    util.promisify = (f) => {
        return function(...args) {
            return new Promise((resolve, reject) => {
                f(...args, (err, value) => {
                    if (error!=null) {
                        reject(err);
                    } else {
                        resolve(value);
                    }
                });
            })
        }
    }
    let path = require('path');
    
    window.__electrico = window.__electrico || {libs:{}};
    function wrapInvoke(invoke) {
        return {"action":"Node", invoke:invoke};
    }
    let node = {
        path:null,
        path: path, 
        fs: {
            constants: {
                "F_OK": 1,
                "R_OK": 2,
                "W_OK": 4,
                "X_OK": 8,
            },
            accessSync(path, mode) {
                const req = createCMDRequest(false);
                req.send(JSON.stringify(wrapInvoke({"command":"FSAccess", "path":path, "mode": mode!=null?mode:1})));
                if (req.responseText!="OK") throw "file access failed: "+path;
            },
            access(path, mode, cb) {
                if (cb==null) {
                    cb = mode;
                    mode=null;
                } 
                const req = createCMDRequest(false);
                req.onreadystatechange = function() {
                    if (this.readyState == 4) {
                        if (req.status == 200) {
                            if (req.responseText=="OK") {
                                cb();
                            } else {
                                cb("file access failed: "+path);
                            }
                        }
                    }
                };
                req.send(JSON.stringify(wrapInvoke({"command":"FSAccess", "path":path, "mode": mode!=null?mode:1})));
            },
            lstatSync(path) {
                const req = createCMDRequest(false);
                req.send(JSON.stringify(wrapInvoke({"command":"FSLstat", "path":path})));
                let resp = JSON.parse(req.responseText);
                return {
                    isDirectory: () => {
                        return resp.isDirectory
                    },
                    isFile: () => {
                        return !resp.isDirectory
                    },
                    birthtime: resp.birthtime!=null?new Date(resp.birthtime.secs_since_epoch*1000):null,
                    mtime: resp.mtime!=null?new Date(resp.mtime.secs_since_epoch*1000):null
                };
            },
            existsSync(path) {
                const req = createCMDRequest(false);
                req.send(JSON.stringify(wrapInvoke({"command":"FSAccess", "path":path, "mode": 1})));
                return req.responseText=="OK";
            },
            exists(path, mode, cb) {
                if (cb==null) {
                    cb = mode;
                    mode=null;
                } 
                const req = createCMDRequest(false);
                req.onreadystatechange = function() {
                    if (this.readyState == 4) {
                        if (req.status == 200) {
                            if (req.responseText=="OK") {
                                cb(true);
                            } else {
                                cb(false);
                            }
                        }
                    }
                };
                req.send(JSON.stringify(wrapInvoke({"command":"FSAccess", "path":path, "mode": 1})));
            },
            mkdirSync(path, options) {
                if (options!=null && typeof options != 'object') options = {recursive: options};
                const req = createCMDRequest(false);
                req.send(JSON.stringify(wrapInvoke({"command":"FSMkdir", "path":path, options:options})));
                if (req.status != 200) throw "mkdir failed: "+path;
                return req.responseText;
            },
            mkdir(path, options, cb) {
                if (cb==null) {
                    cb = options;
                    options=null;
                }
                if (options!=null && typeof options != 'object') options = {recursive: options};
                const req = createCMDRequest(true);
                req.onreadystatechange = function() {
                    if (this.readyState == 4) {
                        if (req.status == 200) {
                            cb(req.responseText);
                        } else throw "mkdir failed: "+path;
                    }
                };
                req.send(JSON.stringify(wrapInvoke({"command":"FSMkdir", "path":path, options:options})));
            },
            writeFileSync(path, data, options) {
                if (options!=null && typeof options != 'object') options = {encoding: options};
                const req = createCMDRequest(false);
                if (options==null || options.encoding==null) {
                    data = btoa(data);
                }
                req.send(JSON.stringify(wrapInvoke({"command":"FSWriteFile", "path":path, "data": data, options:options})));
            },
            writeFile(path, data, options, cb) {
                if (cb==null) {
                    cb = options;
                    options=null;
                }
                if (options!=null && typeof options != 'object') options = {encoding: options};
                const req = createCMDRequest(true);
                req.onreadystatechange = function() {
                    if (this.readyState == 4) {
                        if (req.status == 200) {
                            cb();
                        }
                    }
                };
                if (options==null || options.encoding==null) {
                    data = btoa(data);
                }
                req.send(JSON.stringify(wrapInvoke({"command":"FSWriteFile", "path":path, "data": data, options:options})));
            },
            readFileSync(path, options) {
                if (options!=null && typeof options != 'object') options = {encoding: options};
                const req = createCMDRequest(false);
                req.send(JSON.stringify(wrapInvoke({"command":"FSReadFile", "path":path, options:options})));
                if (options==null || options.encoding==null) {
                    return Buffer.from(req.response);
                }
                return req.responseText;
            },
            readFile(path, options, cb) {
                if (cb==null) {
                    cb = options;
                    options=null;
                }
                if (options!=null && typeof options != 'object') options = {encoding: options};
                const req = createCMDRequest(true);
                req.onreadystatechange = function() {
                    if (this.readyState == 4) {
                        if (req.status == 200) {
                            if (options==null || options.encoding==null) {
                                cb(null, Buffer.from(req.response));
                            } else {
                                cb(null, req.responseText);
                            }
                        } else {
                            cb(req.responseText);
                        }
                    }
                };
                req.send(JSON.stringify(wrapInvoke({"command":"FSReadFile", "path":path, options:options})));
            },
            readdirSync(path, options) {
                if (options!=null && typeof options != 'object') options = {encoding: options};
                const req = createCMDRequest(false);
                req.send(JSON.stringify(wrapInvoke({"command":"FSReadDir", "path":path, options:options})));
                let dirents = JSON.parse(req.responseText);
                if (options==null || !options.withFileTypes) {
                    let names = [];
                    for (let de of dirents) {
                        names.push(de.name);
                    }
                    return names;
                }
                return dirents;
            },
            open(path, flags, mode, cb) {
                if (cb==null) {
                    if (mode!=null) {
                        cb=mode; mode=null;
                    } else {
                        cb=flags; flags=null;
                    }
                }
                if (mode==null) mode="0o666";
                if (flags==null) flags="r";
                _fd++;
                const req = createCMDRequest(true);
                req.send(JSON.stringify(wrapInvoke({"command":"FSOpen", fd:_fd, "path":path, "flags":flags.toLowerCase(), "mode":mode})));
                req.onreadystatechange = function() {
                    if (this.readyState == 4) {
                        if (req.status == 200) {
                            cb(null, req.responseText*1);
                        } else {
                            cb(req.responseText);
                        }
                    }
                };
            },
            close(fd, cb) {
                const req = createCMDRequest(true);
                req.send(JSON.stringify(wrapInvoke({"command":"FSClose", "fd":fd})));
                req.onreadystatechange = function() {
                    if (this.readyState == 4) {
                        if (req.status != 200 && cb!=null) {
                            cb(req.responseText);
                        }
                    }
                };
            },
            read(fd, ...args) {
                let buffer, offset=0, length, position, cb;
                if (args.length==5) {
                    buffer=args[0]; offset=args[1]; length=args[2]; position=args[3]; cb=args[4]; 
                } else {
                    let options=null;
                    if (args.length==3) {
                        buffer=args[0];
                        options=args[1];
                        cb=args[2];
                    } else if (Buffer.isBuffer(args[0])) {
                        buffer=args[0];
                        cb=args[1];
                    } else {
                        options=args[0];
                        cb=args[1];
                    }
                    if (options!=null) {
                        offset=options.offset || offset; length=options.length || length; position=options.position || position;
                        if (buffer==null && options.buffer!=null) buffer=options.buffer;
                    }
                    length = buffer.byteLength-offset;
                }
                const req = createCMDRequest(true);
                req.send(JSON.stringify(wrapInvoke({"command":"FSRead", "fd":fd, "offset":offset, "length":length, "position":position})));
                req.onreadystatechange = function() {
                    if (this.readyState == 4) {
                        if (req.status == 200) {
                            let br = Buffer.from(req.response);
                            let bytesRead = Math.min(br.byteLength, buffer.byteLength);
                            br.copy(buffer, 0, 0, bytesRead);
                            cb(null, bytesRead, buffer);
                        } else {
                            cb(req.responseText);
                        }
                    }
                };
            },
            write(fd, ...args) {
                let buffer, offset=0, length, position, cb;
                let options=null;
                if (Buffer.isBuffer(args[0])) {
                    buffer=args[0];
                    if (args.length>2) {
                        if (typeof (args[1] === 'object')) {
                            options=args[1];
                        } else {
                            offset=args[1];
                        }
                        if (args.length>3) {
                            length = args[2];
                            if (args.length>4) {
                                position = args[3];
                            }
                        }
                    }
                } else {
                    buffer=Buffer.from(args[0], args.length==4?args[2]:(args.length==3?args[1]:'utf-8'));
                    if (args.length==4) position=args[1];
                }
                cb=args[args.length-1];
                if (options!=null) {
                    offset=options.offset || offset; length=options.length || length; position=options.position || position;
                }
                length = buffer.byteLength-offset;
                
                let data = buffer.toString('base64');
                const req = createCMDRequest(true);
                req.send(JSON.stringify(wrapInvoke({"command":"FSWrite", "fd":fd, "data": data,"offset":offset, "length":length, "position":position})));
                req.onreadystatechange = function() {
                    if (this.readyState == 4) {
                        if (req.status == 200) {
                            let written = req.responseText*1;
                            cb(null, written, args[0]);
                        } else {
                            cb(req.responseText);
                        }
                    }
                };
            },
            watch(path, options, cb) {
                const req = createCMDRequest(false);
                let wid = uuidv4();
                req.send(JSON.stringify(wrapInvoke({"command":"FSWatch", wid:wid, "path":path, options:options})));
                if (req.responseText.startsWith("Error: ")) {
                    throw "fs.watch error: "+req.responseText.substring(7);
                }
                class WatcherCls extends EventEmitter {
                    constructor() {
                        super();
                        this.on_event = (eventType, filename) => {
                            let mEventType = null;
                            if (eventType.startsWith("Modify(Name(")) {
                                mEventType = "rename";
                            } else if (eventType.startsWith("Create(")) {
                                mEventType = "change";
                            } else if (eventType.startsWith("Modify(Data(")) {
                                mEventType = "change";
                            } else if (eventType.startsWith("Modify(Any)")) {
                                mEventType = "change";
                            } else if (eventType.startsWith("Modify(Metadata(Extended))")) {
                                mEventType = "change";
                            }
                            if (mEventType!=null) {
                                this.emit("change", mEventType, filename);
                                if (cb!=null) {
                                    cb(eventType, filename);
                                }
                            }
                        }
                        this.close = () => {
                            const req = createCMDRequest(true);
                            req.send(JSON.stringify(wrapInvoke({"command":"FSWatchClose", "wid":wid})));
                        }
                    }
                }
                let watcher = new WatcherCls();
                window.__electrico.fs_watcher[wid] = watcher;
                return watcher;
            },
            promises: {
                stat: (path) => {
                    return new Promise((resolve, reject)=>{
                        resolve(node.fs.lstatSync(path));
                    });
                },
                readdir: (path) => {
                    return new Promise((resolve, reject)=>{
                        resolve(node.fs.readdirSync(path));
                    });
                },
                mkdir: (path, options) => {
                    return new Promise((resolve, reject)=>{
                        resolve(node.fs.mkdirSync(path, options));
                    });
                },
                readFile: (path, options) => {
                    return new Promise((resolve, reject)=>{
                        resolve(node.fs.readFileSync(path, options));
                    });
                }
            }
        },
        http: {
            request(options, cb) {
                let req_events =  {};
                let resp_events =  {};
                let req = createCMDRequest(true);
                req.onreadystatechange = function() {
                    if (this.readyState == 4) {
                        if (cb!=null) {
                            cb({
                                statusCode: req.status,
                                headers: req.getAllResponseHeaders().split("\r\n"),
                                on: (event, cb) => {
                                    resp_events[event] = cb;
                                }
                            });
                        }
                        if (req.status == 200) {
                            if (resp_events["data"]!=null) {
                                resp_events["data"](req.response);
                            }
                        } else if (req_events["error"]!=null) {
                            req_events["error"]("error status "+req.status);
                        }
                    }
                };
                req.error = function(e) {
                    if (req_events["error"]!=null) {
                        req_events["error"]("error "+e);
                    }
                }
                return {
                    on: (event, cb) => {
                        req_events[event] = cb;
                    },
                    end: () => {
                        req.send(JSON.stringify(wrapInvoke({"command":"HTTPRequest", options:options})));
                    }
                }
            }
        },
        child_process: {
            spawn: function(cmd, args, options) {
                let req = createCMDRequest(false);
                req.send(JSON.stringify(wrapInvoke({"command":"ChildProcessSpawn", cmd:cmd, args:args})));
                if (req.responseText.startsWith("Error: ")) {
                    throw "child_process.spawn error: "+req.responseText.substring(7);
                }
                let pid = req.responseText;
                let proc = {
                    pid: pid,
                    on: {},
                    stdout_on: {},
                    stderr_on: {},
                    stdin: {
                        write: (data) => {
                            let req = createCMDRequest(false);
                            req.send(JSON.stringify(wrapInvoke({"command":"ChildProcessStdinWrite", pid: pid, data:data})));
                            if (req.responseText!="OK") {
                                throw "child_process.stdin.write error: "+req.responseText;
                            }
                        }
                    },
                    stdout: {
                        on: (event, cb) => {
                            proc.stdout_on[event] = cb;
                        }
                    },
                    stderr: {
                        on: (event, cb) => {
                            proc.stderr_on[event] = cb;
                        }
                    },
                    on: (event, cb) => {
                        proc.on[event] = cb;
                    },
                    disconnect: () => {
                        let req = createCMDRequest(false);
                        req.send(JSON.stringify(wrapInvoke({"command":"ChildProcessDisconnect", pid: pid})));
                        if (req.responseText!="OK") {
                            throw "child_process.disconnect error: "+req.responseText;
                        }
                    }
                };
                window.__electrico.child_process[pid] = proc;
                return proc;
            }
        },
        os: {
            homedir: () => {
                if (window.__electrico.homedir==null) {
                    const req = createCMDRequest(false);
                    req.send(JSON.stringify({"action":"Electron", invoke:{"command":"GetAppPath", "path":"userHome"}}));
                    window.__electrico.homedir = req.responseText;
                }
                return window.__electrico.homedir;
            },
            tmpdir: () => {
                if (window.__electrico.tmpdir==null) {
                    const req = createCMDRequest(false);
                    req.send(JSON.stringify({"action":"Electron", invoke:{"command":"GetAppPath", "path":"temp"}}));
                    window.__electrico.tmpdir = req.responseText;
                }
                return window.__electrico.tmpdir;
            }
        },
        querystring: queryString,
        util: util,
        events: EventEmitter,
        url: {
            fileURLToPath: (file) => {
                return file;
            }
        },
        module: {
            createRequire: (file) => {
                return require;
            },
            register: (script, path) => {

            }
        },
        crypto: {
            createHash: (alg) => {
                if (alg=="sha256") {
                    let SHA256 = require("crypto-js/sha256");
                    return {
                        update: (text) => {
                            let hash = SHA256(text);
                            return {
                                digest: (d) => {
                                    if (d=="hex") {
                                        return hash.toString();
                                    } else {
                                        throw "createHash - unknown digest: "+d;
                                    }
                                }
                            }
                        }
                    }
                } else {
                    throw "createHash - unknown algorithm: "+alg;
                }
            }
        },
        net: {
            // TODO
            Server: {
                
            },
            Socket: {
            },
            createServer: {

            },
            createConnection: {

            }
        },
        zlib :{
            // TODO
            createDeflateRaw: {

            },
            ZlibOptions: {

            },
            InflateRaw: {

            },
            DeflateRaw: {

            },
            createInflateRaw: {

            }
        }
    };
    window.__electrico.libs["node:path"] = node.path;
    window.__electrico.libs.path = node.path;
    window.__electrico.libs["node:fs"] = node.fs;
    window.__electrico.libs.fs = node.fs;
    window.__electrico.libs["node:child_process"] = node.child_process;
    window.__electrico.libs.child_process = node.child_process;
    window.__electrico.libs["node:https"] = node.http;
    window.__electrico.libs.https = node.http;
    window.__electrico.libs["node:http"] = node.http;
    window.__electrico.libs.http = node.http;
    window.__electrico.libs["node:os"] = node.os;
    window.__electrico.libs.os = node.os;
    window.__electrico.libs["node:querystring"] = node.querystring;
    window.__electrico.libs.querystring = node.querystring;
    window.__electrico.libs["node:util"] = node.util;
    window.__electrico.libs.util = node.util;
    window.__electrico.libs["node:events"] = node.events;
    window.__electrico.libs.events = node.events;
    window.__electrico.libs["node:url"] = node.url;
    window.__electrico.libs.url = node.url;
    window.__electrico.libs["node:module"] =node.module;
    window.__electrico.libs.module = node.module;
    window.__electrico.libs["node:crypto"] =node.crypto;
    window.__electrico.libs.crypto = node.crypto;
    window.__electrico.libs["node:net"] =node.net;
    window.__electrico.libs.net = node.net;
    window.__electrico.libs["node:zlib"] =node.zlib;
    window.__electrico.libs.zlib = node.zlib;
})();