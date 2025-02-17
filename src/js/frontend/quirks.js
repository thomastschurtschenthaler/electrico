(function() {
    let protocols = null;
    function getProtocols() {
        if (protocols!=null) return protocols;
        const req = new XMLHttpRequest();
        req.open("POST", window.__create_protocol_url(__create_ipc_url("send")), false);
        req.send(JSON.stringify({"action": "GetProtocols"}));
        protocols = JSON.parse(req.responseText);
        return protocols;
    }
    function toLocalHTTPURL(url) {
        if (url==null) return null;
        for (let p of getProtocols()) {
            if (url.startsWith(p+"://")) {
                let ix = url.indexOf(":");
                return "http://"+window.location.hostname+":"+window.location.port+"/"+window.__http_protocol.http_uid+"@"+url.substring(0, ix)+"/"+url.substring(ix+3);
            }
        }
        return url;
    }
    let _URL = window.URL;
    window.URL = class extends window.URL {
        constructor(uri, baseUrl) {
            if (baseUrl!=null) {
                baseUrl = new _URL(toLocalHTTPURL(baseUrl.toString()));
            }
            super(uri, baseUrl);
        }
    };
    let _setAttribute=HTMLElement.prototype.setAttribute;
    HTMLElement.prototype.setAttribute=function(a, v){
        if (a=="src") {
            if (v!=null) {
                for (let p of getProtocols()) {
                    if (v.startsWith(p+"://")) {
                        let ix = v.indexOf(":");
                        let url_post = v.substring(ix+3);
                        let protocol = v.substring(0, ix);
                        let url = protocol+"/"+url_post;
                        if (this instanceof HTMLIFrameElement) {
                            window.__custom_iframe_protocol = protocol;
                            v = "http://"+url_post.substring(0, url_post.indexOf("/"))+".localhost:"+window.location.port+"/"+window.__http_protocol.http_uid+"@"+url;
                        } else {
                            v = "http://"+window.location.hostname+":"+window.location.port+"/"+window.__http_protocol.http_uid+"@"+url;
                        }
                        break;
                    }
                }
            }
        }
        _setAttribute.bind(this)(a, v);
    };
    let _fetch = window.fetch;
    window.fetch = (function(url, ...args) {
        if (url.startsWith!=null) {
            url = toLocalHTTPURL(url);
        }
        return _fetch.bind(window)(url, ...args);
    }).bind(window);
    let _stringify = JSON.stringify;
    JSON.stringify = (obj, ...args) => {
        if (obj!=null && obj.startsWith!=null) {
            obj = toLocalHTTPURL(obj);
        }
        return _stringify(obj, ...args);
    };
    replaceStyleURLs = () => {
        let styles = document.querySelectorAll("style");
        styles.forEach(style => {
            for (let p of getProtocols()) {
                if (style.textContent.indexOf(p+"\\:\\/\\/")>=0) {
                    style.textContent = style.textContent.replaceAll(p+"\\:\\/\\/", "http://"+window.location.hostname+":"+window.location.port+"/"+window.__http_protocol.http_uid+"@"+p+"/");
                }
            }
        });
        setTimeout(replaceStyleURLs, 2000);
    }
    setTimeout(replaceStyleURLs, 2000);
})();