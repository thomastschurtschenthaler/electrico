(function() {
    let uuidv4 = window.__uuidv4;
    let PTY = {
        spawn: (shell, args, opt) => {
            console.log("PTY.spawn", shell, args, opt);
            opt = opt || {};
            args = args || [];
            opt.name = opt.name || "term";
            opt.rows = opt.rows || 30;
            opt.cols = opt.cols || 80;
            opt.cwd = opt.cwd || window.__dirname;
            opt.env = opt.env || {};
            let id = uuidv4();
            let {r, e} = $e_node.syncAddon_PTY_Spawn({"id":id, "shell":shell, "args":args, "opt":opt});
            if (e!=null) {
                throw "PTY.spawn: "+e;
            }
            let ptyProcess = {
                onData: (cb) => {
                    window.__electrico.child_process[id] = {
                        stdout_on: {
                            data: (data) => {
                                cb(data);
                            }
                        }
                    }
                },
                write: (data) => {
                    let {r, e} = $e_node.syncChildProcessStdinWrite({pid: id}, data);
                    if (e!=null) {
                        throw "ptyProcess.write error: "+e;
                    }
                },
                resize: (cols, rows) => {
                    // TODO
                    console.log("ptyProcess.resize", cols, rows);
                }
            }
            
            return ptyProcess;
        }
    }
    window.__electrico.libs["node-pty"] = PTY;
})();