#![deny(warnings)]
use std::str::FromStr;
use bytes::Bytes;
use clap::{Command,arg};
use http::{Method, StatusCode};
use http_body_util::Empty;
use http_body_util::{combinators::BoxBody,BodyExt};
use hyper::client::conn::http1::Builder;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper::header::{HeaderName,HeaderValue};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

static mut HOST_ADDR:String = String::new();
static mut TARGET_ADDR:String = String::new();
static mut ALLOW_HEADER:String = String::new();
static mut ALLOW_PRINT_LOG:bool = false;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {


    let matches = Command::new("web_proxy")
    .version("1.0")
    .about("设置http代理")
    .author("git")
    .arg(arg!(--h <VALUE>).required(true))
    .arg(arg!(--t <VALUE>).required(true))
    .arg(arg!(--headers <VALUE>).required(false))
    .arg(arg!(--log).required(false))
    .get_matches();

    unsafe{
      ALLOW_HEADER = matches.get_one::<String>("headers").map_or("".to_string(), |v|v.to_string());
      HOST_ADDR = matches.get_one::<String>("h").map_or("".to_string(), |v|v.to_string());
      TARGET_ADDR = matches.get_one::<String>("t").map_or("*".to_string(), |v|v.to_string());
      ALLOW_PRINT_LOG = matches.get_one::<bool>("log").map_or(false, |v|*v);
    }

    println!("代理地址---  {:#?}",unsafe {HOST_ADDR.as_str()});
    println!("目标地址---  {:#?}",unsafe {TARGET_ADDR.as_str()});

    // let addr = SocketAddr::from(HOST_ADDR.as_str());

    let addr_parse = Url::parse(unsafe {
        HOST_ADDR.as_str()
    }).unwrap();
    let addr = format!("{}:{}",addr_parse.host().unwrap(),addr_parse.port().map_or(80, |v|v));

    let listener = TcpListener::bind(addr).await?;
    // println!("Listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .preserve_header_case(true)
                .title_case_headers(true)
                .serve_connection(stream, service_fn(proxy))
                .with_upgrades()
                .await
            {
                println!("Failed to serve connection: {:?}", err);
            }
        });
    }
}

async fn proxy(
    mut req: Request<hyper::body::Incoming>,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    if *req.method() == Method::OPTIONS{
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "POST, PUT, GET, OPTIONS, DELETE")
            .header("Access-Control-Allow-Headers", unsafe { ALLOW_HEADER.as_str() })
            .body(empty())
            .unwrap())
    }else{
        // println!("转发请求");
        // let addr = format!("{}:{}", "127.0.0.1", "8001");
            
        let addr_parse = Url::parse(unsafe {
            TARGET_ADDR.as_str()
        }).unwrap();
        let addr = format!("{}:{}",addr_parse.host().unwrap(),addr_parse.port().map_or(80, |v|v));

        let stream = TcpStream::connect(addr).await.unwrap();
    
        let (mut sender, conn) = Builder::new()
            .title_case_headers(true)
            .handshake(stream)
            .await?;
            tokio::task::spawn(async move {
                if let Err(err) = conn.await {
                    println!("Connection failed: {:?}", err);
                }
            });
            
        // let proxy_request = Request::builder()
        // .header("Host", "www.baidu.com")
        // .method(req.method())
        // .body(Empty::<Bytes>::new()).unwrap();
        req.headers_mut().insert(HeaderName::from_str("Host").unwrap(), HeaderValue::from_str(&format!("{}",addr_parse.host().unwrap())).unwrap());
        req.headers_mut().remove(HeaderName::from_str("Origin").unwrap()).unwrap();
        if unsafe { ALLOW_PRINT_LOG }{
            println!("request:--------开始\n {:#?} \n--------结束", req);
        }
        // println!("proxy_request: {:?} ", proxy_request);

        let mut resp = sender.send_request(req).await?;
        let headers = resp.headers_mut();
        let _ = headers.insert(HeaderName::from_str("Access-Control-Allow-Origin").unwrap(), HeaderValue::from_str("*").unwrap());
        let _ = headers.insert(HeaderName::from_str("Access-Control-Allow-Methods").unwrap(), HeaderValue::from_str("POST, PUT, GET, OPTIONS, DELETE").unwrap());
        let _ = headers.insert(HeaderName::from_str("Access-Control-Allow-Headers").unwrap(), HeaderValue::from_str(unsafe { ALLOW_HEADER.as_str() }).unwrap());
        if unsafe { ALLOW_PRINT_LOG }{
            println!("response:--------开始\n {:#?} \n--------结束", &resp);
        }
        

        Ok(resp.map(|b| b.boxed()))
    }

        
    
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}