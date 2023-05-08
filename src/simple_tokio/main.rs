use std::{net::{SocketAddr}, sync::{Arc,Mutex}};
use tokio::{net::{TcpListener,TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}}, io::{AsyncWriteExt, AsyncReadExt,BufReader, BufWriter}};
use clap::{arg,Command};

async fn handle_connection(stream: TcpStream,target_addr:SocketAddr) {
  let (reader, writer) = stream.into_split();
  let client_reader = BufReader::new(reader);
  let client_writer = BufWriter::new(writer);

  let server_stream = TcpStream::connect(target_addr).await.unwrap();
  let (reader, writer) = server_stream.into_split();
  let server_reader = BufReader::new(reader);
  let server_writer = BufWriter::new(writer);

  let is_option_req = Arc::new(Mutex::new(false));

  let read_client_status = is_option_req.clone();
  let read_server_status = is_option_req.clone();

  println!("proxy to server...");

  tokio::join!(read_client(client_reader, server_writer,read_client_status), read_server(server_reader, client_writer,read_server_status));
  println!("request end...");
}


async fn read_client(mut client_reader: BufReader<OwnedReadHalf>, mut server_writer: BufWriter<OwnedWriteHalf>,read_client_status:Arc<Mutex<bool>>) {
  let mut buffer = [0; 1024];
  loop {
      let read_result = client_reader.read(&mut buffer).await;
      if let Ok(length) = read_result {
          // println!("报文读取长度{}----------",length);
          if length > 0 {
              let end_res_string = String::from_utf8_lossy(&buffer[0..length]).to_string();
              // println!("原始的请求---{}\n",&end_res_string);
              println!("锁的状态,{}",read_client_status.lock().unwrap());
              if end_res_string.starts_with("OPTIONS") {
                let mut temp = read_client_status.lock().unwrap();
                *temp= true;
                break;
              }
              println!("锁的状态  更改后,{}",*read_client_status.lock().unwrap());
              let content = &buffer[0..length];
              let _ = server_writer.write(content).await;
              let _ = server_writer.flush().await;
              
          } else {
              // println!("client close ...");
              break;
          }
          
      }
  }
  // println!("读取结束----------");
}

async fn read_server(mut server_reader: BufReader<OwnedReadHalf>, mut client_writer: BufWriter<OwnedWriteHalf>,read_server_status:Arc<Mutex<bool>>) {
  let mut buffer = [0; 1024];

  loop {
      println!("锁的状态{}---\n",*read_server_status.lock().unwrap());
      if *read_server_status.lock().unwrap(){
        let mut reponse_string = String::from("HTTP/1.1 200 OK");
        reponse_string.push_str("\r\nAccess-Control-Allow-Origin: *");
        reponse_string.push_str("\r\nAccess-Control-Allow-Methods: POST, PUT, GET, OPTIONS, DELETE");
        reponse_string.push_str("\r\nAccess-Control-Allow-Headers: ");
        reponse_string.push_str(unsafe { &ALLOW_HEADER });
        reponse_string.push_str("\r\nAccess-Control-Max-Age: 10");
        let _ = client_writer.write(reponse_string.as_bytes()).await;
        let _ = client_writer.flush().await;
        println!("客户端写入内容");
        break;
      }
      println!("继续写入");
      let read_result = server_reader.read(&mut buffer).await;
      if let Ok(length) = read_result {
          if length > 0 {
              
              let end_res_string = String::from_utf8_lossy(&buffer[0..length]).to_string();
              if !end_res_string.contains("Access-Control-Allow-Origin:") && end_res_string.contains("HTTP"){
                // let first_rn = end_res_string.find("\r\n").unwrap();
                // println!("索引位置{}----",first_rn);

                // end_res_string.insert_str(first_rn, "\r\nAccess-Control-Allow-Origin: *");
                // end_res_string.insert_str(first_rn, "\r\nAccess-Control-Allow-Methods: POST, PUT, GET, OPTIONS, DELETE");
                // end_res_string.insert_str(first_rn, &allow_headers);
                // end_res_string.insert_str(first_rn, "\r\n");

                // for it in "Access-Control-Allow-Origin: *".chars().into_iter(){
                //     end_res_string.insert(first_rn, it);
                // }
                
                // for it in "Access-Control-Allow-Methods: POST, PUT, GET, OPTIONS, DELETE".chars().into_iter(){
                //     end_res_string.insert(first_rn, it);
                // }
                // for it in allow_headers.as_str().chars().into_iter(){
                //     end_res_string.insert(first_rn, it);
                // }
                // let _ = client_writer.write(&end_res_string.as_bytes()).await;
                let mut index = 0; // 判断换行符位置
                for (i,it) in buffer.iter().enumerate(){
                  if it == &b'\n'{
                    index = i;
                    break;
                  }
                }
                let mut content = buffer[0..length].to_vec();
                for (i,it) in "\r\nAccess-Control-Allow-Origin: *".bytes().into_iter().enumerate(){
                  content.insert(index+i, it);
                }
                for (i,it) in "\r\nAccess-Control-Allow-Methods: POST, PUT, GET, OPTIONS, DELETE".bytes().into_iter().enumerate(){
                  content.insert(index+i, it);
                }
                let mut allow_header_string = "Access-Control-Allow-Headers: ".to_string();
                allow_header_string.push_str(unsafe { &ALLOW_HEADER });
                for (i,it) in allow_header_string.as_str().bytes().into_iter().enumerate(){
                  content.insert(index+i, it);
                }
                // let end_res_string = String::from_utf8_lossy(&content[..]).to_string();
                // println!("转发的响应----{}",end_res_string);
                let _ = client_writer.write(&content[..]).await;
              }else{
                let content = &buffer[0..length];
                let _ = client_writer.write(content).await;
              }
              // let _ = client_writer.write(&end_res_string.as_bytes()).await;
              // println!("转发的报文");
              // println!("原始的响应{}",&end_res_string);
              // println!("\n----------");
              let _ = client_writer.flush().await;
          } else {
              // println!("server close ...");
              break;
          }
      }
  }
}

async fn create_proxy(host_http:String,target_http:String){

  if host_http.is_empty() || target_http.is_empty(){
    println!("缺少参数");
    return;
  }

  println!("服务启动");
  println!("代理地址---  {:#?}",host_http);
  println!("目标地址---  {:#?}",target_http);

  let host_http_addr:SocketAddr = host_http.parse().unwrap();
  let target_http_addr:SocketAddr = target_http.parse().unwrap();

  let listener = TcpListener::bind(host_http_addr).await.unwrap();

  loop {
      
      let (stream, _) = listener.accept().await.unwrap();
      tokio::spawn(async move {
          handle_connection(stream,target_http_addr).await;
      });
  }
}

static mut ALLOW_HEADER:String = String::new();

#[tokio::main]
async fn main() {
    let matches = Command::new("web_proxy")
    .version("1.0")
    .about("设置http代理")
    .author("git")
    .arg(arg!(--h <VALUE>).required(true))
    .arg(arg!(--t <VALUE>).required(true))
    .arg(arg!(--headers <VALUE>).required(false))
    .get_matches();

    unsafe{
      ALLOW_HEADER = matches.get_one::<String>("headers").map_or("*".to_string(), |v|v.to_string());
    }

    create_proxy(
        matches.get_one::<String>("h").map_or("".to_string(), |v|v.to_string()),
        matches.get_one::<String>("t").map_or("".to_string(), |v|v.to_string()),
    ).await;
}
