(function() {
    window.__init_require = function (window) {
        //console.trace("__init_require call", window);
        function fromCache(expanded_path) {
            return window.__electrico.module_cache[expanded_path];
        }
        function loadModule(_this, mpath, cache) {
            let lib = window.__electrico.getLib(mpath, __electrico_nonce);
            if (lib!=null) {
                return lib;
            }
            var module = {}; var exports = {};
            let module_path = _this!=null?_this.__import_mpath:window.__create_protocol_url("fil://mod");
            let expanded_path = module_path;
            if (mpath.startsWith(".")) {
                expanded_path+="/"+mpath;
            } else {
                expanded_path=window.__create_protocol_url("fil://mod/node_modules/"+mpath);
            }
            
            let cached = fromCache(expanded_path);
            if (cached!=null && cached!="" && cache) {
               return cached;
            }
            let script=null; let req={};
            if (cached!="") {
                let jsfilepath = (expanded_path.lastIndexOf(".")<expanded_path.lastIndexOf("/"))?expanded_path+".js":expanded_path;
                req = new XMLHttpRequest();
                req.open("GET", jsfilepath, false);
                req.send();
            }
            if (cached=="" || req.status==301) {
                //console.trace("js file not found", expanded_path);
                window.__electrico.module_cache[expanded_path]="";
                let package_path = expanded_path+"/package.json";
                const preq = new XMLHttpRequest();
                preq.open("GET", package_path, false);
                preq.send();
                if (preq.status==301) {
                    console.error("js file not found - no package.json", package_path);
                    return null;
                }
                let package = JSON.parse(preq.responseText);
                let mainjs = package.main!=null?package.main:(package.exports!=null?(package.exports.default!=null?package.exports.default:package.exports):package.files[0]);
                expanded_path = expanded_path+"/"+mainjs;
                
                if (!expanded_path.endsWith("js")) expanded_path+=".js";
                if (cache) {
                    let cached = fromCache(expanded_path);
                    if (cached!=null) {
                        return cached;
                    }
                }
                const req2 = new XMLHttpRequest();
                req2.open("GET", expanded_path, false);
                req2.send();
                if (req2.status==404) {
                    console.error("js file not found", expanded_path);
                    return null;
                }
                script=req2.responseText;
            } else {
                script=req.responseText;
            }
            let exported = null;
            if (mpath.endsWith(".json")) {
                exported = JSON.parse(script);
            } else {
                let _this = {"__import_mpath":expanded_path.substring(0, expanded_path.lastIndexOf("/"))};
                eval("//# sourceURL="+expanded_path.substring(10, expanded_path.length) +"\n{\nlet __require_this=_this;"+window.__replaceImports(script)+"\n}");
                exported = module.exports || exports;
            }
            if (cache) {
                window.__electrico.module_cache[expanded_path]=exported;
            }
            return exported;
        }
        window.__Import=function(_this, mpath, selector) {
            //console.log("__import", mpath, selector);
            let mod = loadModule(_this, mpath, false);
            if (selector!=null) {
                //console.log("selector mod", mod);
                /*let modsel = {};
                if (selector=="*") {
                    for (let k in mod)
                }*/
            }
            return mod; 
        }
        window.__importinline=function(__require_this, mpath) {
            return new Promise((resolve, reject) => {
                let mod = loadModule(__require_this, mpath, false);
                if (mod==null) {
                    reject();
                } else {
                    resolve(mod);
                }
            });
        }
        window.require=function(__require_this, mpath) {
            if (mpath==null) {
                mpath = __require_this;
                __require_this=null;
            }
            return loadModule(__require_this, mpath, true);
        }
        window.__replaceImports = (script) => {
            let impr = script.replaceAll(/\import *((.*) +as +)?(.*) *from *([^{ ,;,\r, \n}]*)/g, "var $3 = __Import(__require_this, $4, '$2')").replaceAll("import.meta", "__Import_meta");
            impr = impr.replaceAll(/( |\n|\r)import *([^{ ,;,\r, \n}]*);/g, "__Import(__require_this, $2, '')");
            impr = impr.replaceAll("import(", "__importinline(__require_this, ");
            impr = impr.replaceAll("require(", "require(__require_this, ");
            impr = impr.replaceAll(/\export +(default)?(const)?([^ ]* +([^{ ,(,;,\n}]*))/g, "exports['$4']=$3").replaceAll("'use strict'", "").replaceAll('"use strict"', "");
            return impr;
        }
    };
})();