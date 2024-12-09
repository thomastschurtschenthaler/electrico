(function() {
    function electricoStore(cb) {
        const dbo = window.indexedDB.open("electricoStore", 1);
        dbo.onupgradeneeded = function() {
            const db = dbo.result;
            db.createObjectStore("electricoStore", {keyPath: "id"});
        };
        dbo.onsuccess = function() {
            const db = dbo.result;
            const tx = db.transaction("electricoStore", "readwrite");
            const store = tx.objectStore("electricoStore");
            cb(null, store);
            tx.oncomplete = function() {
                db.close();
            };
        }
        dbo.onerror = function(e) {
            cb(e);
        }
    }
    function retrieveKey(cb) {
        try {
            electricoStore(function (e, store) {
                if (e!=null) {
                    cb(e);
                    return;
                }
                const data = store.get(1);
                data.onsuccess = async function() {
                    try {
                        let result = data.result;
                        if (result!=null) {
                            cb(null, result.key);
                            return;
                        }
                    } catch (e) {
                        cb("retrieveKey error:"+e);
                        return;
                    }
                    const newKey = await crypto.subtle.generateKey(
                        { name: "AES-GCM", length: 256 },
                        false,
                        ['encrypt', 'decrypt']
                    );
                    let put = store.put({id: 1, key: newKey});
                    put.onsuccess = function() {
                        cb(null, newKey);
                    };
                    put.onerror = function(e) {
                        cb(e);
                    }
                };
                data.onerror = function(e) {
                    cb(e);
                }
            });
        } catch (e) {
            cb(e);
        }
    }
    const { spawn } = require('node:child_process');
    let safeStorage = {
        isEncryptionAvailableAsync: async function() {
            return new Promise((res, rej)=> {
                retrieveKey((e, r)=> {
                    res(e==null && r!=null);
                });
            });
        },
        encryptStringAsync: async function(text) {
            return new Promise((res, rej)=> {
                retrieveKey(async (e, key)=> {
                    if (e!=null) {
                        rej(e); return;
                    }
                    if (key==null) {
                        rej("encryptStringAsync - key is null"); return;
                    }
                    const encoded = new TextEncoder().encode(text);
                    const iv = crypto.getRandomValues(new Uint8Array(12));
                    const ciphertext = await crypto.subtle.encrypt({name:"AES-GCM", iv:iv}, key, encoded);
                    const buf = Buffer.concat([Buffer.from(iv), Buffer.from(ciphertext)]);
                    res(buf.toString("base64"));
                });
            });
        },
        decryptStringAsync: async function(bufferbase64) {
            let buffer = Buffer.from(bufferbase64, "base64");
            return new Promise((res, rej)=> {
                retrieveKey(async (e, key)=> {
                    if (e!=null) {
                        rej(e); return;
                    }
                    if (key==null) {
                        rej("decryptStringAsync - key is null"); return;
                    }
                    const iv = buffer.subarray(0, 12);
                    const ebuffer = buffer.subarray(12, buffer.length);
                    const cleartext = await crypto.subtle.decrypt({name:"AES-GCM", iv:iv}, key, ebuffer);
                    const text = new TextDecoder().decode(cleartext);
                    res(text);
                });
            });
        },
        isEncryptionAvailable: () => {
            let script = 'let _req=require;_req("electron").safeStorage.isEncryptionAvailableAsync();';
            let child = spawn(process.execPath, ["-p", script]);
            let {r, e} = $e_node.syncGetDataBlobBin({"id":child.pid+"stdout", "timeoutms":600000});
            if (e!=null) return false;
            let res = Buffer.from(r).toString();
            return res=="true";
        },
        encryptString: function(text) {
            let script = 'let _req=require;_req("electron").safeStorage.encryptStringAsync(\''+text.replaceAll("'", "\'").replaceAll("\r", "\\r").replaceAll("\n", "\\n")+'\');';
            let child = spawn(process.execPath, ["-p", ""]);
            child.stdin.write(script);
            let {r, e} = $e_node.syncGetDataBlobBin({"id":child.pid+"stdout", "timeoutms":600000});
            if (e!=null) throw "encryptString error: "+e;
            r = Buffer.from(r).toString();
            if (r.trim().length==0) throw "encryptString error: no key";
            let res = Buffer.from(r, "base64");
            return res;
        },
        decryptString: function(buffer) {
            let bufferbase64 = buffer.toString("base64");
            let script = 'let _req=require;_req("electron").safeStorage.decryptStringAsync(\''+bufferbase64+'\');';
            let child = spawn(process.execPath, ["-p", ""]);
            child.stdin.write(script);
            let {r, e} = $e_node.syncGetDataBlobBin({"id":child.pid+"stdout", "timeoutms":600000});
            if (e!=null) throw "decryptString error: "+e;
            r = Buffer.from(r).toString();
            if (r.trim().length==0) throw "decryptString error: no key";
            return r;
        },
    };
    window.__electrico.libs["electron"].safeStorage = safeStorage;
})();