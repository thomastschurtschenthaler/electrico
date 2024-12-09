(function() {
    const uuidv4 = window.__uuidv4;
    const EventEmitter = require('eventemitter3');
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
            class PcpCls extends EventEmitter {
                constructor() {
                    super();
                    this.stdout = new EventEmitter();
                    this.stderr = new EventEmitter();
                }
            }
            let pcp = new PcpCls();
            window.__electrico.child_process[id] = pcp;
            let ptyProcess = {
                pid:parseInt(r),
                onData: (cb) => {
                    pcp.stdout.on("data", (data) => {
                        cb({data:data});
                    });
                },
                write: (data) => {
                    let {r, e} = $e_node.syncApi_Childprocess_StdinWrite({pid: id}, data);
                    if (e!=null) {
                        throw "ptyProcess.write error: "+e;
                    }
                },
                resize: (cols, rows) => {
                    // TODO
                    console.error("ptyProcess.resize", cols, rows);
                },
                onExit: (cb) => {
                    pcp.on("close", (exit_code)=> {
                        cb(exit_code);
                    });
                }
            }
            
            return ptyProcess;
        }
    }
    window.__electrico.libs["node-pty"] = PTY;
})();