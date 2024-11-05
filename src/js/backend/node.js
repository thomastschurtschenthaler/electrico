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
                f(...args, (err, ...value) => {
                    if (err!=null) {
                        reject(err);
                    } else {
                        resolve(...value);
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
    let decoder = new TextDecoder();
    let node = {
        path: path, 
        fs: {
            constants: {
                "F_OK": 1,
                "R_OK": 2,
                "W_OK": 4,
                "X_OK": 8,
            },
            accessSync(path, mode) {
                let {r, e} = $e_node.syncFSAccess({"path":path, "mode": mode!=null?mode:1});
                if (e!=null) throw "file access failed: "+path;
            },
            access(path, mode, cb) {
                if (cb==null) {
                    cb = mode;
                    mode=null;
                }
                $e_node.asyncFSAccess({"path":path, "mode": mode!=null?mode:1}).then((e, r)=>{
                    if (e!=null) {
                        cb("file access failed: "+path);
                    } else {
                        cb();
                    }
                });
            },
            lstatSync(path) {
                let {r, e} = $e_node.syncFSLstat({"path":path});
                if (e!=null) throw {"code":"ENOENT", "message": "no such file or directory, lstat '"+path+"'"};
                let resp = JSON.parse(r);
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
                let {r, e} = $e_node.syncFSAccess({"path":path, "mode": 1});
                return r=="OK";
            },
            exists(path, mode, cb) {
                if (cb==null) {
                    cb = mode;
                    mode=null;
                }
                $e_node.asyncFSAccess({"path":path, "mode": 1}).then((e, r)=>{
                    if (r=="OK") {
                        cb(true);
                    } else {
                        cb(false);
                    }
                });
            },
            mkdirSync(path, options) {
                if (options!=null && typeof options != 'object') options = {recursive: options};
                let {r, e} = $e_node.syncFSMkdir({"path":path, options:options});
                if (e!=null) throw "mkdir failed: "+path;
                return r;
            },
            mkdir(path, options, cb) {
                if (cb==null) {
                    cb = options;
                    options=null;
                }
                if (options!=null && typeof options != 'object') options = {recursive: options};
                $e_node.asyncFSMkdir({"path":path, options:options}).then((e, r)=>{
                    if (e!==null) {
                        throw "mkdir failed: "+path;
                    } else {
                        cb(null, r);
                    }
                });
            },
            writeFileSync(path, data, options) {
                if (options!=null && typeof options != 'object') options = {encoding: options};
                if (typeof path === 'number') {
                    node.fs.write(path, data, ()=>{});
                    return {};
                }
                let {r, e} = $e_node.syncFSWriteFile({"path":path, options:options}, data);
                if (e!=null) throw "writeFileSync failed: "+path;
            },
            writeFile(path, data, options, cb) {
                if (cb==null) {
                    cb = options;
                    options=null;
                }
                if (options!=null && typeof options != 'object') options = {encoding: options};
                if (typeof path === 'number') {
                    node.fs.write(path, data, cb);
                    return;
                }
                $e_node.asyncFSWriteFile({"path":path, "data": data, options:options}, data).then((e, r)=>{
                    cb(e);
                });
            },
            readFileSync(path, options) {
                if (options!=null && typeof options != 'object') options = {encoding: options};
                let {r, e} = $e_node.syncFSReadFileBin({"path":path, options:options});
                if (e!=null) throw "readFileSync failed: "+path;
                if (options==null || options.encoding==null) {
                    return Buffer.from(r);
                }
                return decoder.decode(r);
            },
            readFile(path, options, cb) {
                if (cb==null) {
                    cb = options;
                    options=null;
                }
                if (options!=null && typeof options != 'object') options = {encoding: options};
                $e_node.asyncFSReadFileBin({"path":path, options:options}).then((e, r)=>{
                    if (e!==null) {
                        cb(e);
                    } else {
                        if (options==null || options.encoding==null) {
                            cb(null, Buffer.from(r));
                        } else {
                            cb(null, decoder.decode(r));
                        }
                    }
                });
            },
            readdirSync(path, options) {
                if (options!=null && typeof options != 'object') options = {encoding: options};
                let {r, e} = $e_node.syncFSReadDir({"path":path, options:options});
                if (e!=null) throw "readdirSync failed: "+path;
                let dirents = JSON.parse(r);
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
                $e_node.asyncFSOpen({fd:_fd, "path":path, "flags":flags.toLowerCase(), "mode":mode+""}).then((e, r)=>{
                    if (e!==null) {
                        cb(e);
                    } else {
                        cb(null, r*1);
                    }
                });
            },
            close(fd, cb) {
                $e_node.asyncFSClose({"fd":fd}).then((e, r)=>{
                    cb(e);
                });
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
                $e_node.asyncFSRead({"fd":fd, "offset":offset, "length":length, "position":position}).then((e, r)=>{
                    if (e!==null) {
                        cb(e);
                    } else {
                        let br = Buffer.from(r);
                        let bytesRead = Math.min(br.byteLength, buffer.byteLength);
                        br.copy(buffer, 0, 0, bytesRead);
                        cb(null, bytesRead, buffer);
                    }
                });
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
                $e_node.asyncFSWrite({"fd":fd,"offset":offset, "length":length, "position":position}, buffer).then((e, r)=>{
                    if (e!==null) {
                        cb(e);
                    } else {
                        let written = r*1;
                        cb(null, written, args[0]);
                    }
                });
            },
            realpath: (path, options, cb) => {
                if (cb==null) {
                    cb = options;
                    options=null;
                }
                let {r, e} = $e_node.syncFSRealPath({"path":path});
                if (e!=null) throw "realpath failed: "+path;
                cb(null, r);
            },
            fdatasync: (fd, cb) => {
                $e_node.asyncFSFdatasync({"fd":fd}).then((e, r) => {
                    cb(e);
                });
            },
            unlink: (path) => {
                $e_node.syncFSUnlink({"path":path});
            },
            renameSync: (oldPath, newPath) => {
                $e_node.syncFSRename({"old_path":oldPath, "new_path": newPath});
            },
            watch(path, options, cb) {
                let wid = uuidv4();
                let {r, e} = $e_node.syncFSWatch({wid:wid, "path":path, options:options});
                if (e!=null) {
                    throw "fs.watch error: "+e;
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
                            $e_node.asyncFSWatchClose({wid:wid});
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
                },
                unlink: (path) => {
                    return new Promise((resolve, reject)=>{
                        resolve(node.fs.unlink(path));
                    });
                },
                rename: (oldPath, newPath) => {
                    return new Promise((resolve, reject)=>{
                        resolve(node.fs.renameSync(oldPath, newPath));
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
                let {r, e} = $e_node.syncChildProcessSpawn({cmd:cmd, args:args});
                if (e!=null) {
                    throw "child_process.spawn error: "+e;
                }
                let pid = r;
                let proc = {
                    pid: pid,
                    on: {},
                    stdout_on: {},
                    stderr_on: {},
                    stdin: {
                        write: (data) => {
                            let {r, e} = $e_node.syncChildProcessStdinWrite({pid: pid}, data);
                            if (e!=null) {
                                throw "child_process.stdin.write error: "+e;
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
                        let {r, e} = $e_node.syncChildProcessDisconnect({pid: pid});
                        if (e!=null) {
                            throw "child_process.disconnect error: "+e;
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
                    let {r, e} = $e_electron.syncGetAppPath({ "path":"userHome"});
                    window.__electrico.homedir = r;
                }
                return window.__electrico.homedir;
            },
            tmpdir: () => {
                if (window.__electrico.tmpdir==null) {
                    let {r, e} = $e_electron.syncGetAppPath({ "path":"temp"});
                    window.__electrico.tmpdir = r;
                }
                return window.__electrico.tmpdir;
            },
            release: () => {
                return "darwin";
            },
            hostname: () => {
                return "localhost";
            },
            arch: () => {
                return "arm";
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
                let calgr;
                if (alg=="sha256") {
                    calgr = require("crypto-js/sha256");
                } else if (alg=="md5") {
                    calgr = require("crypto-js/md5");
                } else {
                    throw "createHash - unknown algorithm: "+alg;
                }
                return {
                    update: (text) => {
                        let hash = calgr(text);
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
            }
        },
        net: {
            createServer: function(options, listener) {
                if (listener==null) {
                    listener=options;
                    options=null;
                }
                class ServerCls extends EventEmitter {
                    constructor() {
                        super();
                        this._connections={};
                        this.listen = ((hook, cb) => {
                            if (cb!=null) {
                                this.on("listening", cb);
                            }
                            let {r, e} = $e_node.syncNETCreateServer({"hook":hook, "options":options});
                            if (e==null) {
                                window.__electrico.net_server[hook]=this;
                                this.id=r;
                                this.emit("listening");
                            } else {
                                this.emit("error", e);
                            }
                        }).bind(this);
                        this.close = ((cb) => {
                            for (let cid in this._connections) {
                                let {r, e} = $e_node.syncNETCloseConnection({"id":cid});
                                this._connections[cid].emit("close");
                                delete window.__electrico.net_server[cid];
                            }
                            let {r, e} = $e_node.syncNETCloseServer({"id":this.id});
                            this._connections={};
                            for (let id in window.__electrico.net_server) {
                                if (window.__electrico.net_server[id]==this) {
                                    delete window.__electrico.net_server[id];
                                }
                            }
                            this.emit("close");
                            if (cb!=null) cb();
                        }).bind(this);
                        this._connection_start = (id => {
                            class ConnectionCls extends EventEmitter {
                                constructor(server) {
                                    super();
                                    server._connections[id] = this;
                                    this.write = ((data, encoding, cb) => {
                                        if (cb==null) {
                                            cb=encoding;
                                            encoding=null;
                                        }
                                        encoding = encoding || 'utf-8';
                                        if (!Buffer.isBuffer(data)) {
                                            data=Buffer.from(data, encoding);
                                        }
                                        $e_node.asyncNETWriteConnection({"id":id}, data).then((e, r)=>{
                                            if (cb!=null) cb(e==null);
                                        });
                                    }).bind(this);
                                    this.end = ((data, encoding, cb) => {
                                        cb = cb || encoding;
                                        let end = () => {
                                            setTimeout(()=>{
                                                let {r, e} = $e_node.syncNETCloseConnection({"id":id});
                                            }, 100);
                                        };
                                        if (data!=null) {
                                            this.write(data, encoding, ()=>{
                                                end();
                                            });
                                        } else {
                                            end();
                                        }
                                    }).bind(this);
                                    this._connection_end = (id => {
                                        this.emit("end");
                                        delete server._connections[id];
                                        delete window.__electrico.net_server[id];
                                    }).bind(this);
                                }
                            }
                            let connection = new ConnectionCls(this);
                            window.__electrico.net_server[id] = connection;
                            this.emit("connection", connection);
                        }).bind(this);
                    }
                }
                let server = new ServerCls();
                if (listener!=null) {
                    server.on("connection", listener);
                }
                return server;
            },
            createConnection: function (hook, listener) {
                class ConnectionCls extends EventEmitter {
                    constructor() {
                        super();
                        this.write = ((data, encoding, cb) => {
                            if (cb==null) {
                                cb=encoding;
                                encoding=null;
                            }
                            encoding = encoding || 'utf-8';
                            if (!Buffer.isBuffer(data)) {
                                data=Buffer.from(data, encoding);
                            }
                            $e_node.asyncNETWriteConnection({"id":id}, data).then((e, r)=>{
                                if (cb!=null) cb(e==null);
                            });
                        }).bind(this);
                        this.end = ((data, encoding, cb) => {
                            cb = cb || encoding;
                            let end = () => {
                                setTimeout(()=>{
                                    let {r, e} = $e_node.syncNETCloseConnection({"id":id});
                                }, 100);
                            };
                            if (data!=null) {
                                this.write(data, encoding, ()=>{
                                    end();
                                });
                            } else {
                                end();
                            }
                        }).bind(this);
                        this.setTimeout = ((t => {
                            $e_node.asyncNETSetTimeout({"id":id, "timeout":t});
                        })).bind(this);
                        this._connection_end = (id => {
                            this.emit("end");
                            delete window.__electrico.net_server[id];
                        }).bind(this);
                        this._connection_timeout = (id => {
                            this.emit("timeout");
                        }).bind(this);
                    }
                }
                let id = uuidv4();
                let connection = new ConnectionCls();
                if (listener!=null) {
                    connection.on("connect", listener);
                }
                window.__electrico.net_server[id] = connection;
                let {r, e} = $e_node.syncNETCreateConnection({"id":id, "hook":hook});
                if (e!=null) {
                    console.error("createConnection error: ", e);
                    setTimeout(()=>{
                        connection.emit("error", e);
                    }, 0);
                } else {
                    setTimeout(()=>{
                        connection.emit("connect");
                    }, 0);
                }
                return connection;
            }
        },
        zlib :{
            createDeflateRaw: (options) => {
                class DeflateRawCls extends EventEmitter {
                    constructor(options) {
                        super();
                        this.write = (async function(data) {
                            let stream = new Blob([data]).stream();
                            const cReader = stream.pipeThrough(new CompressionStream("gzip")).getReader();
                            while (true) {
                                let read = await cReader.read();
                                if (read.done){
                                    this.emit("end");
                                    if (this._fluscb!=null) {
                                        this._fluscb();
                                    }
                                    break;
                                } else {
                                    this.emit("data", Buffer.from(read.value));
                                }
                            }
                        }).bind(this);
                        this.flush = (cb => {
                            this._fluscb=cb;
                        }).bind(this);
                    }
                }
                let deflate = new DeflateRawCls(options);
                return deflate; 
            },
            createInflateRaw: (options) => {
                class InflateRawCls extends EventEmitter {
                    constructor(options) {
                        super();
                        this.write = (async function(data) {
                            let stream = new Blob([data]).stream();
                            const cReader = stream.pipeThrough(new DecompressionStream("gzip")).getReader();
                            while (true) {
                                let read = await cReader.read();
                                if (read.done){
                                    this.emit("end");
                                    if (this._fluscb!=null) {
                                        this._fluscb();
                                    }
                                    break;
                                } else {
                                    this.emit("data", Buffer.from(read.value));
                                }
                            }
                        }).bind(this);
                        this.flush = (cb => {
                            this._fluscb=cb;
                        }).bind(this);
                    }
                }
                let inflate = new InflateRawCls(options);
                return inflate; 
            }
        },
        assert: {

        },
        stream: {
            Transform: class {

            }
        },
        readline: {
            createInterface: (options) => {
                // TODO
                console.log("readline.createInterface");
                return null;
            }
        }
    };
    
    window.__electrico.libs["node:path"] = node.path;
    window.__electrico.libs.path = node.path;
    window.__electrico.libs["node:fs"] = node.fs;
    window.__electrico.libs["original-fs"] = node.fs;
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
    window.__electrico.libs["node:assert"] =node.assert;
    window.__electrico.libs.assert = node.assert;
    window.__electrico.libs["node:stream"] =node.stream;
    window.__electrico.libs.stream = node.stream;
    window.__electrico.libs["node:readline"] =node.readline;
    window.__electrico.libs.readline = node.readline;
})();