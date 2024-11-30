(function() {
    function new_process(pid) {
        let proc = {
            pid: pid,
            on: {},
            stdout_on: {},
            stderr_on: {},
            stdin: {
                write: (data) => {
                    let {r, e} = $e_node.syncApi_Childprocess_StdinWrite({pid: pid}, data);
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
                console.error("process.disconnect", pid);
                let {r, e} = $e_node.syncApi_Childprocess_Disconnect({pid: pid});
                if (e!=null) {
                    throw "child_process.disconnect error: "+e;
                }
            },
            kill: (signal) => {
                console.error("process.kill", pid);
                let {r, e} = $e_node.syncApi_Childprocess_Kill({pid: pid});
            }
        };
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