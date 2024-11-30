(function() {
    let EventEmitter = require('eventemitter3');
    let uuidv4 = window.__uuidv4;
    let decoder = new TextDecoder();
    let _fd=0;
    let fs = {
        constants: {
            "F_OK": 1,
            "R_OK": 2,
            "W_OK": 4,
            "X_OK": 8,
        },
        accessSync(path, mode) {
            let {r, e} = $e_node.syncApi_FS_Access({"path":path, "mode": mode!=null?mode:1});
            if (e!=null) throw "file access failed: "+path;
        },
        access(path, mode, cb) {
            if (cb==null) {
                cb = mode;
                mode=null;
            }
            $e_node.asyncApi_FS_Access({"path":path, "mode": mode!=null?mode:1}).then((e, r)=>{
                if (e!=null) {
                    cb("file access failed: "+path);
                } else {
                    cb();
                }
            });
        },
        lstat(path, mode, cb) {
            if (cb==null) {
                cb = mode;
                mode=null;
            }
            $e_node.asyncApi_FS_Lstat({"path":path}).then((e, r)=>{
                if (e!=null) {
                    cb({"code":"ENOENT", "message": "no such file or directory, lstat '"+path+"'"});
                } else {
                    let resp = JSON.parse(r);
                    cb(null, {
                        isDirectory: () => {
                            return resp.isDirectory
                        },
                        isFile: () => {
                            return !resp.isDirectory
                        },
                        birthtime: resp.birthtime!=null?new Date(resp.birthtime.secs_since_epoch*1000):null,
                        mtime: resp.mtime!=null?new Date(resp.mtime.secs_since_epoch*1000):null
                    });
                }
            });
        },
        lstatSync(path) {
            let {r, e} = $e_node.syncApi_FS_Lstat({"path":path});
            if (e!=null) {
                throw {"code":"ENOENT", "message": "no such file or directory, lstat '"+path+"'"};
            }
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
            let {r, e} = $e_node.syncApi_FS_Access({"path":path, "mode": 1});
            return r=="OK";
        },
        exists(path, mode, cb) {
            if (cb==null) {
                cb = mode;
                mode=null;
            }
            $e_node.asyncApi_FS_Access({"path":path, "mode": 1}).then((e, r)=>{
                if (r=="OK") {
                    cb(true);
                } else {
                    cb(false);
                }
            });
        },
        mkdirSync(path, options) {
            if (options!=null && typeof options != 'object') options = {recursive: options};
            let {r, e} = $e_node.syncApi_FS_Mkdir({"path":path, options:options});
            if (e!=null) throw "mkdir failed: "+path;
            return r;
        },
        mkdir(path, options, cb) {
            if (cb==null) {
                cb = options;
                options=null;
            }
            if (options!=null && typeof options != 'object') options = {recursive: options};
            $e_node.asyncApi_FS_Mkdir({"path":path, options:options}).then((e, r)=>{
                if (e!==null) {
                    throw "mkdir failed: "+path;
                } else {
                    cb(null, r);
                }
            });
        },
        rm(path, options, cb) {
            if (cb==null) {
                cb = options;
            }
            $e_node.asyncApi_FS_Rm({"path":path}).then((e, r)=>{
                if (e!=null) {
                    console.error("asyncFSRm failed: "+path);
                    cb(e);
                }
            });
        },
        rmSync(path, options) {
            let {r, e} = $e_node.syncApi_FS_Rm({"path":path, options:options}, data);
            if (e!=null) throw "asyncFSRm failed: "+path;

        },
        writeFileSync(path, data, options) {
            if (options!=null && typeof options != 'object') options = {encoding: options};
            if (typeof path === 'number') {
                fs.write(path, data, ()=>{});
                return {};
            }
            let {r, e} = $e_node.syncApi_FS_WriteFile({"path":path, options:options}, data);
            if (e!=null) throw "writeFileSync failed: "+path;
        },
        writeFile(path, data, options, cb) {
            if (cb==null) {
                cb = options;
                options=null;
            }
            if (options!=null && typeof options != 'object') options = {encoding: options};
            if (typeof path === 'number') {
                fs.write(path, data, cb);
                return;
            }
            $e_node.asyncApi_FS_WriteFile({"path":path, "data": data, options:options}, data).then((e, r)=>{
                cb(e);
            });
        },
        readFileSync(path, options) {
            if (options!=null && typeof options != 'object') options = {encoding: options};
            let {r, e} = $e_node.syncApi_FS_ReadFileBin({"path":path, options:options});
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
            $e_node.asyncApi_FS_ReadFileBin({"path":path, options:options}).then((e, r)=>{
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
            let {r, e} = $e_node.syncApi_FS_ReadDir({"path":path, options:options});
            if (e!=null) throw "readdirSync failed: "+path;
            let dirents = JSON.parse(r);
            if (options==null || !options.withFileTypes) {
                let names = [];
                for (let de of dirents) {
                    names.push(de.path+"/"+de.name);
                }
                return names;
            }
            let entries = dirents.map(de=>{return {
                name:de.name,
                path:de.path,
                isDirectory: () => {
                    return de.isDirectory;
                },
                isFile: () => {
                    return !de.isDirectory;
                },
                isSymbolicLink: () => {
                    return false;
                }
            }});
            return entries;
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
            $e_node.asyncApi_FS_Open({fd:_fd, "path":path, "flags":flags.toLowerCase(), "mode":mode+""}).then((e, r)=>{
                if (e!==null) {
                    cb(e);
                } else {
                    cb(null, r*1);
                }
            });
        },
        close(fd, cb) {
            $e_node.asyncApi_FS_Close({"fd":fd}).then((e, r)=>{
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
            $e_node.asyncApi_FS_Read({"fd":fd, "offset":offset, "length":length, "position":position}).then((e, r)=>{
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
            $e_node.asyncApi_FS_Write({"fd":fd,"offset":offset, "length":length, "position":position}, buffer).then((e, r)=>{
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
            let {r, e} = $e_node.syncApi_FS_RealPath({"path":path});
            if (e!=null) throw "realpath failed: "+path;
            cb(null, r);
        },
        fdatasync: (fd, cb) => {
            $e_node.asyncApi_FS_Fdatasync({"fd":fd}).then((e, r) => {
                cb(e);
            });
        },
        unlink: (path) => {
            $e_node.asyncApi_FS_Unlink({"path":path});
        },
        renameSync: (oldPath, newPath) => {
            $e_node.syncApi_FS_Rename({"old_path":oldPath, "new_path": newPath});
        },
        watch(path, options, cb) {
            let wid = uuidv4();
            let {r, e} = $e_node.syncApi_FS_Watch({wid:wid, "path":path, options:options});
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
                        $e_node.asyncApi_FS_WatchClose({wid:wid});
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
                    fs.lstat(path, (e, r)=>{
                        if (e!=null) {
                            reject(e);
                        } else {
                            resolve(r);
                        }
                    });
                });
            },
            access: (path, mode) => {
                return new Promise((resolve, reject)=>{
                    fs.access(path, mode, (e) => {
                        if (e!=null) {
                            reject(e);
                        } else {
                            resolve();
                        }
                    })
                });
            },
            readdir: (path, options) => {
                return new Promise((resolve, reject)=>{
                    resolve(fs.readdirSync(path, options));
                });
            },
            mkdir: (path, options) => {
                return new Promise((resolve, reject)=>{
                    resolve(fs.mkdirSync(path, options));
                });
            },
            rm: (path, options) => {
                return new Promise((resolve, reject)=>{
                    resolve(fs.rm(path, options, (e) => {
                        if (e!=null) {
                            reject(e);
                        } else {
                            resolve();
                        }
                    }));
                });
            },
            readFile: (path, options) => {
                return new Promise((resolve, reject)=>{
                    resolve(fs.readFileSync(path, options));
                });
            },
            unlink: (path) => {
                return new Promise((resolve, reject)=>{
                    resolve(fs.unlink(path));
                });
            },
            rename: (oldPath, newPath) => {
                return new Promise((resolve, reject)=>{
                    resolve(fs.renameSync(oldPath, newPath));
                });
            }
        }
    };
    window.__electrico.libs["node:fs"] = fs;
    window.__electrico.libs["original-fs"] = fs;
    window.__electrico.libs.fs = fs;
})();