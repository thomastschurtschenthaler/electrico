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
        tls: {
            createSecureContext: (options) => {
                console.log("tls.createSecureContext", options);
                return {}
            }
        },
        tty: {
            isatty: (fd) => {
                console.error("tty.isatty", fd);
                return true;
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

    require("./apis/apis.js");
    require("./addons/addons.js");
})();