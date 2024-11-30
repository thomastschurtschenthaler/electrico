(function() {
    let EventEmitter = require('eventemitter3');
    let uuidv4 = window.__uuidv4;
    let net = {
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
                        let {r, e} = $e_node.syncApi_NET_CreateServer({"hook":hook, "options":options});
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
                            let {r, e} = $e_node.syncApi_NET_CloseConnection({"id":cid});
                            this._connections[cid].emit("close");
                            delete window.__electrico.net_server[cid];
                        }
                        let {r, e} = $e_node.syncApi_NET_CloseServer({"id":this.id});
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
                                    $e_node.asyncApi_NET_WriteConnection({"id":id}, data).then((e, r)=>{
                                        if (cb!=null) cb(e==null);
                                    });
                                }).bind(this);
                                this.end = ((data, encoding, cb) => {
                                    cb = cb || encoding;
                                    let end = () => {
                                        setTimeout(()=>{
                                            let {r, e} = $e_node.syncApi_NET_CloseConnection({"id":id});
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
                        $e_node.asyncApi_NET_WriteConnection({"id":id}, data).then((e, r)=>{
                            if (cb!=null) cb(e==null);
                        });
                    }).bind(this);
                    this.end = ((data, encoding, cb) => {
                        cb = cb || encoding;
                        let end = () => {
                            setTimeout(()=>{
                                let {r, e} = $e_node.syncApi_NET_CloseConnection({"id":id});
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
                        $e_node.asyncApi_NET_SetTimeout({"id":id, "timeout":t});
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
            let {r, e} = $e_node.syncApi_NET_CreateConnection({"id":id, "hook":hook});
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
    };
    window.__electrico.libs["node:net"] = net;
    window.__electrico.libs.net = net;
})();