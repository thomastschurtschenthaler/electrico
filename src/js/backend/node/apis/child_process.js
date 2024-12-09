(function() {
    const EventEmitter = require('eventemitter3');
    function new_process(pid) {
        class ProcCls extends EventEmitter {
            constructor(pid) {
                super();
                this.pid = pid;
                this.stdin = {
                    write: (data) => {
                        let {r, e} = $e_node.syncApi_Childprocess_StdinWrite({pid: pid}, data);
                        if (e!=null) {
                            throw "child_process.stdin.write error: "+e;
                        }
                    }
                };
                this.stdout = new EventEmitter();
                this.stderr = new EventEmitter();
                this.disconnect = () => {
                    console.log("process.disconnect", pid);
                    let {r, e} = $e_node.syncApi_Childprocess_Disconnect({pid: pid});
                    if (e!=null) {
                        throw "child_process.disconnect error: "+e;
                    }
                },
                this.kill = (signal) => {
                    console.log("process.kill", pid);
                    let {r, e} = $e_node.syncApi_Childprocess_Kill({pid: pid});
                }
            }
        }
        let proc = new ProcCls(pid);
        window.__electrico.child_process[pid] = proc;
        return proc;
    }
    let child_process = {
        spawn: function(cmd, args, options) {
            let {r, e} = $e_node.syncApi_Childprocess_Spawn({cmd:cmd, args:args});
            if (e!=null) {
                throw "child_process.spawn error: "+e;
            }
            let pid = r;
            let proc = new_process(pid);
            return proc;
        },
        exec: function(cmd, options, cb) {
            if (cb==null) {
                cb=options;
                options=null;
            }
            let {r, e} = $e_node.syncApi_Childprocess_Spawn({});
            if (e!=null) {
                throw "child_process.exec error: "+e;
            }
            let pid = r;
            let proc = new_process(pid);
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
        }
    };
    window.__electrico.libs["node:child_process"] = child_process;
    window.__electrico.libs.child_process = child_process;
})();