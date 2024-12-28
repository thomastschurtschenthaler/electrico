(function() {
    const EventEmitter = require('eventemitter3');
    function new_process(pid) {
        class Readable extends EventEmitter {
            constructor(proc) {
                super();
                this.setEncoding = (encoding) => {
                    console.log("Readable.setEncoding", encoding);
                }
            }
        }
        class ProcCls extends EventEmitter {
            constructor(pid) {
                super();
                this.pid = pid;
                this.stdin = {
                    write: function(chunk, encoding, callback, end) {
                        end = end || false;
                        if (callback==null && typeof encoding === 'function') {
                            callback=encoding; encoding=null;
                        }
                        if (encoding!=null) {
                            chunk = Buffer.from(chunk, encoding);
                        }
                        let {r, e} = $e_node.syncApi_Childprocess_StdinWrite({pid: pid, end: end}, chunk);
                        if (e!=null) {
                            throw "child_process.stdin.write error: "+e;
                        }
                        if (callback!=null) {
                            callback();
                        }
                    },
                    end: (function(chunk, encoding, callback) {
                        if (callback==null && typeof encoding === 'function') {
                            callback=encoding; encoding=null;
                        }
                        if (chunk!=null) {
                            this.stdin.write(chunk, encoding, null, true);
                        }
                        if (callback!=null) {
                            callback();
                        }
                    }).bind(this)
                };
                this.stdout = new Readable(this);
                this.stderr = new Readable(this);
                this.disconnect = () => {
                    console.log("process.disconnect", pid);
                    let {r, e} = $e_node.syncApi_Childprocess_Disconnect({pid: pid});
                    if (e!=null) {
                        throw "child_process.disconnect error: "+e;
                    }
                },
                this.kill = (signal) => {
                    let {r, e} = $e_node.syncApi_Childprocess_Kill({pid: pid});
                    return e==null;
                }
            }
        }
        let proc = new ProcCls(pid);
        window.__electrico.child_process[pid] = proc;
        return proc;
    }
    let child_process = {
        spawn: function(cmd, args, options) {
            let {r, e} = $e_node.syncApi_Childprocess_Spawn({cmd:cmd, args:args, options:options});
            if (e!=null) {
                throw "child_process.spawn error: "+e;
            }
            let pid = r;
            let proc = new_process(pid, cmd);
            return proc;
        },
        exec: function(cmd, options, cb) {
            if (cb==null) {
                cb=options;
                options=null;
            }
            let {r, e} = $e_node.syncApi_Childprocess_Spawn({options:options});
            if (e!=null) {
                throw "child_process.exec error: "+e;
            }
            let pid = r;
            let proc = new_process(pid, cmd);
            if (cb!=null) {
                let stdout="";let stderr="";
                proc.stdout.on('data', (data) => {
                    stdout+=data.toString();
                });
                proc.stderr.on('data', (data) => {
                    stderr+=data.toString();
                });
                proc.on('close', (code) => {
                    cb(code!=0?code:null, stdout, stderr);
                });
            }
            proc.stdin.write(cmd+"\nexit\n");
            return proc;
        },
        fork: function(modulePath, args, options) {
            let utilityProcess = require("electron").utilityProcess.fork(modulePath, args, options);
            utilityProcess.send = function(message, sendHandle, options, callback) {
                if (callback==null) {
                    callback=sendHandle;
                    sendHandle=null; options=null;
                }
                utilityProcess.postMessage(message);
                if (callback!=null) {
                    callback(null);
                }
            };
            return utilityProcess;
        }
    };
    window.__electrico.libs["node:child_process"] = child_process;
    window.__electrico.libs.child_process = child_process;
})();