(function() {
    const util = require('util');
    const EventEmitter = require('eventemitter3');
    function TransformCls(options) {
        EventEmitter.call(this);
        options = options || {};
        this._readableState = {};
        let _this=this;
        this.write = (data) => {
            if (_this._transform!=null) {
                _this._transform(data, options.encoding, ()=>{
                    
                });
            } else {
                _this.push(data);
            }
        }
        this.push = (data) => {
            _this.emit("data", data);
        }
    }
    util.inherits(TransformCls, EventEmitter);

    const stream = EventEmitter;
    stream.Transform = TransformCls;
    window.__electrico.libs["node:stream"] = stream;
    window.__electrico.libs.stream = stream;
})();