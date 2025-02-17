(function() {
    const request = async function(url, requestOptions) {
        console.log("request:",url);
        let params = {method:requestOptions!=null && requestOptions.method!=null?requestOptions.method:"GET"};
        if (requestOptions.body!=null) params.body=requestOptions.body;
        if (requestOptions.headers!=null) params.headers=requestOptions.headers;
        if (requestOptions.query!=null) {
            url+="?"
            for (let qp in requestOptions.query) {
                url+=(qp+"="+requestOptions.query[qp]+"&");
            }
        }
        let resp = await fetch(url, params);
        let headers = {};
        for (const pair of resp.headers.entries()) {
            headers[pair[0]]=pair[1];
        }
        console.log("headers:",headers);
        return {
            statusCode: ""+resp.status,
            headers:headers,
            body: {
                text:()=>{
                    console.log("request text()");
                    return resp.text();
                },
                json:()=>{
                    console.log("request json()");
                    return resp.json();
                }
            }
        };
    };
    const undici = {
        request: request
    }
    window.__electrico.libs.undici = undici;
})();