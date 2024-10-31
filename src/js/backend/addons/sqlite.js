(function() {
    let EventEmitter = require('eventemitter3');
    class StatementCls extends EventEmitter {
        constructor(db, sql) {
            super();
            this.run=(params, cb) => {
                $e_node.asyncAddon_SQLite_Exec({"cid":db._cid, "sql":sql, "params":params}).then((e, r)=>{
                    if (cb!=null) cb(e, r);
                });
            };
            this.finalize=(cb) => {
                cb();
            };
        }
    }
    class DatabaseCls extends EventEmitter {
        constructor(path, cb) {
            super();
            $e_node.asyncAddon_SQLite_Connect({"path":path}).then((e, r)=>{
                this._cid = r;
                if (cb!=null) cb(e);
            });
            this.close = ((cb) => {
                let {e, r} = $e_node.syncAddon_SQLite_Close({"cid":this._cid});
                if (cb!=null) cb(e, r);    
            }).bind(this);
            this.exec = ((sql, cb) => {
                $e_node.asyncAddon_SQLite_Exec({"cid":this._cid, "sql":sql}).then((e, r)=>{
                    if (cb!=null) cb(e, r);
                });
            }).bind(this);
            this.serialize = (cb => {
                $e_node.asyncAddon_SQLite_Serialize({"cid":this._cid}).then((e, r)=>{
                    if (cb!=null) cb(e, r);
                });
            }).bind(this);
            this.run = ((cmd, cb) => {
                $e_node.asyncAddon_SQLite_Run({"cid":this._cid, "cmd":cmd}).then((e, r)=>{
                    if (cb!=null) cb(e, r);
                });
            }).bind(this);
            this.prepare = (sql => {
                return new StatementCls(this, sql);
            }).bind(this);
            this.get = ((sql, cb) => {
                $e_node.asyncAddon_SQLite_Query({"cid":this._cid, "sql":sql, "all":false}).then((e, r)=>{
                    if (r!=null) {
                        r = JSON.parse(r);
                        r = r.length>0?r[0]:null;
                    }
                    if (cb!=null) cb(e, r);
                });
            }).bind(this);
            this.all = ((sql, cb) => {
                $e_node.asyncAddon_SQLite_Query({"cid":this._cid, "sql":sql, "all":true}).then((e, r)=>{
                    if (r!=null) {
                        r = JSON.parse(r);
                    }
                    if (cb!=null) cb(e, r);
                });
            }).bind(this);
        }
    };

    class BackupCls {
    }
    let SQLite3 = {
        Database: DatabaseCls,
        Statement: StatementCls,
        Backup: BackupCls
    };
    SQLite3.default = SQLite3;
    window.__electrico.libs["vscode-sqlite3.node"] = SQLite3;
})();