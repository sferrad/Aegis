use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    if std::env::args().nth(1).as_deref() == Some("healthcheck"){
        std::process::exit(healthcheck());
    }
    let addr = "0.0.0.0:8080";
    let listener = TcpListener::bind(addr).expect("Failed to bind to address");
    println!("aegis {VERSION} listening on {addr}");
    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            thread::spawn(|| handle(stream));
        }
    }
}

fn healthcheck() -> i32 {
   match TcpStream::connect("127.0.0.1:8080") {
        Ok(mut stream) => {
            let request = "GET /health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
            if stream.write_all(request.as_bytes()).is_ok() {
                let mut buf = [0u8; 1024];
                if let Ok(n) = stream.read(&mut buf) {
                    let response = String::from_utf8_lossy(&buf[..n]);
                    if response.contains("200 OK") {
                        return 0; // Healthy
                    }
                }
            }
            1 // Unhealthy
        }
        Err(_) => 1, // Unhealthy
    }
}

fn handle(mut stream: TcpStream){
    let mut buf = [0u8; 1024];
    let n = match stream.read(&mut buf) {
        Ok(n) => n,
        Err(_) => {
            return;
        }
    };
    let request = String::from_utf8_lossy(&buf[..n]);
    let path = request
                .lines()
                .next()
                .and_then(|l| l.split_whitespace().nth(1))
                .unwrap_or("/");
    
    let (status, body) = match path {
        "/" => ("200 OK", format!("{{\"service\": \"aegis\", \"version\": \"{VERSION}\"}}")),
        "/health" => ("200 OK", "{\"status\": \"healthy\"}".into()),
        _ => ("404 Not Found", "Not Found".into())
    };
    
    let response = format!(
       "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}