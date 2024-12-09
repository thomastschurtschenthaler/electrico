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
    let node = {
        path: path,
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
            },
            platform: () => {
                return process.platform;
            },
            type: () => {
                return process.platform=="darwin"?"Darwin":(process.platform=="linux"?"Linux":"Windows_NT");
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
        timers: {
            setImmediate(cb) {
                setTimeout(cb, 0);
            }
        },
        tls: {
            createSecureContext: (options) => {
                console.log("tls.createSecureContext", options);
                return {}
            }
        },
        tty: {
            isatty: (fd) => {
                return true;
            }
        },
        dns: {

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
        },
        buffer: require("buffer")
    };
    
    window.__electrico.libs["node:path"] = node.path;
    window.__electrico.libs.path = node.path;
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
    window.__electrico.libs["node:zlib"] =node.zlib;
    window.__electrico.libs.zlib = node.zlib;
    window.__electrico.libs["node:assert"] =node.assert;
    window.__electrico.libs.assert = node.assert;
    window.__electrico.libs["node:stream"] =node.stream;
    window.__electrico.libs.stream = node.stream;
    window.__electrico.libs["node:readline"] =node.readline;
    window.__electrico.libs.readline = node.readline;
    window.__electrico.libs["node:tls"] =node.tls;
    window.__electrico.libs.tls = node.tls;
    window.__electrico.libs["node:tty"] =node.tty;
    window.__electrico.libs.tty = node.tty;
    window.__electrico.libs["node:dns"] = node.dns;
    window.__electrico.libs.dns = node.dns;
    window.__electrico.libs["node:timers"] = node.timers;
    window.__electrico.libs.timers = node.timers;
    window.__electrico.libs["node:buffer"] = node.buffer;
    window.__electrico.libs.buffer = node.buffer;

    require("./apis/apis.js");
    require("./addons/addons.js");
})();