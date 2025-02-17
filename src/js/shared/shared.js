(function() {
    function init_shared (window) {
        function getCircularReplacer() {
            const ancestors = [];
            return function (key, value) {
              if (typeof value !== "object" || value === null) {
                return value;
              }
              while (ancestors.length > 0 && ancestors.at(-1) !== this) {
                ancestors.pop();
              }
              if (ancestors.includes(value)) {
                return null;
              }
              ancestors.push(value);
              return value;
            };
        }
        let _stringify = JSON.stringify;
        JSON.stringify = (obj, r) => {
            return _stringify(obj, r!=null?r:getCircularReplacer());
        };
        window.__create_file_url= (path) => {
          if (window.location.protocol=="http:" || window.location.protocol=="https:") {
            return "electrico-file://"+path;
          }
          return window.location.protocol+"//"+path;
      }
      window.__create_protocol_url = (url, sync_cmd) => {
        if ((sync_cmd == null || !sync_cmd) && window.__http_protocol!=null) {
          let ix = url.indexOf(":");
          url = "http://electrico.localhost:"+window.__http_protocol.http_port+"/"+window.__http_protocol.http_uid+"@"+url.substring(0, ix)+"/"+url.substring(ix+3);
        } else if (window.__is_windows) {
          let ix = url.indexOf(":");
          url = "http://"+url.substring(0,ix)+"."+url.substring(ix+3);
        }  
        return url;
      };
      window.__uuidv4 = function() {
        return "10000000-1000-4000-8000-100000000000".replace(/[018]/g, c =>
            (+c ^ window.crypto.getRandomValues(new Uint8Array(1))[0] & 15 >> +c / 4).toString(16)
        );
      };
      window.__init_require(window);

      let ipc_websockets={};
      function ipc_websocket(wschannel, out, nonce, hook, messagehandler, cb) {
        let channel = (hook==null?wschannel+(out?"out":"in"):hook);
        if (cb==null) {
          cb = messagehandler; messagehandler=null;
        }
        if (ipc_websockets[channel]!=null) {
            if (ipc_websockets[channel].socket!=null) {
                cb(ipc_websockets[channel].socket);   
            } else {
                ipc_websockets[channel].cbs.push(cb);
            }
            return;
        }
        ipc_websockets[channel] = {cbs:[cb]};
        let url = hook || "ws://electrico.localhost:"+window.__http_protocol.http_port+"/"+window.__http_protocol.http_uid+"@async"+(out?"out":"in")+"/"+channel+(nonce!=null?"/"+nonce+"/"+channel:"");
        let socket = new WebSocket(url);
        socket.binaryType = "arraybuffer";
        if (messagehandler!=null) {
            socket.addEventListener("message", (event) => {
                if (event.data instanceof ArrayBuffer) {
                    messagehandler(event.data);
                } else {
                    console.error("got text", event.data);
                }
            });
        }
        
        socket.onopen = (event) => {
          console.log("onopen", event);
          ipc_websockets[channel].socket = socket;
          for (let cb of ipc_websockets[channel].cbs) {
              cb(socket);
          }
          delete ipc_websockets[channel].cbs;
        };
        socket.onerror = function(error) {
          console.log("error", error);
        };
      }
      window.__ipc_websocket = ipc_websocket;
      window.__ipc_websocket_messagehandler = function(handler) {
        let message=null;
        return function(data) {
            if (message!=null) {
              handler(message.channel, message.args, Buffer.from(data));
              message=null;
              return;
          }
          let msg = (new TextDecoder()).decode(data);
          let i1 = msg.indexOf("|");
          let i2 = msg.indexOf("|", i1+1);
          let hasData = msg.substring(0, i1)=="true";
          let channel = msg.substring(i1+1, i2);
          let args = msg.substring(i2+1, msg.length);
          if (hasData) {
              message = {channel, args};
              return;
          }
          handler(channel, args);
        };
      };
      let queues = {};
      window.__call_queue = function(qid, cb, ...args) { 
        let q = queues[qid];
        if (q == null) {
          q = {queue:[], start:(new Date()).getTime(), trigger:false};
          queues[qid] = q;
        }
        q.queue.push({args, cb});
        let doCall = function(qid) {
          let q = queues[qid];
          if (q==null) return;
          let ix = 0;
          for (let a of q.queue) {
            let success = a.cb(...a.args);
            if (!success) {
              if ((new Date()).getTime() - q.start<5000) {
                if (!q.trigger) {
                  q.trigger = true;
                  setTimeout(()=>{q.trigger=false; doCall(qid);}, 200);
                }
              } else {
                console.error("window.__call_queue -  timed out", qid);
              }
              q.queue = q.queue.slice(ix, q.queue.length);
              return;
            }
            ix++;
          }
          delete queues[qid];
        }
        doCall(qid);
      };
      if (!window._no_websocket) {
        (function() {
              ipc_websocket("ipc", true, __electrico_nonce, null, 
                window.__ipc_websocket_messagehandler((channel, args, data) => {
                  /*if (data!=null) {
                    let txt = data.toString();
                    console.log("__ipc_websocket_messagehandler channel:"+channel+"; data:"+txt.substring(txt.length-5));
                  }*/
                  window.__electrico.sendChannelMessage(channel, args, data);
                }),
              (socket)=>{
                
              });
        })();
      }
    }
    window.__init_shared = init_shared;
})();