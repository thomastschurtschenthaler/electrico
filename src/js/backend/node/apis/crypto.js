(function() {
    let crypto = {
        createHash: (alg) => {
            let calgr;
            if (alg=="sha256") {
                calgr = require("crypto-js/sha256");
            } else if (alg=="md5") {
                calgr = require("crypto-js/md5");
            } else {
                throw "createHash - unknown algorithm: "+alg;
            }
            let value="";
            let hash = {
                update: (text) => {
                    value+=text;
                    return hash;
                },
                digest: (d) => {
                    let hashv = calgr(value);
                    if (d=="hex") {
                        return hashv.toString();
                    } else {
                        throw "createHash - unknown digest: "+d;
                    }
                }
            };
            return hash;
        },
        getRandomValues: (array) => {
            return window.crypto.getRandomValues(array);
        }
    };
    crypto.webcrypto=crypto;
    window.__electrico.libs["node:crypto"] = crypto;
    window.__electrico.libs.crypto = crypto;
})();