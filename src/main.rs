use std::{
    fs,
    io::{BufReader, ErrorKind, prelude::*},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use rsblog::ThreadPool;

// TODO:
// - read all blog htmls and display them in iframe with route
//   - use filter to ship static files
// - integrate tera (template engine)

fn main() {
    let bind_address: &str = "127.0.0.1:8080";
    let listener: TcpListener = TcpListener::bind(bind_address).unwrap();
    let pool = ThreadPool::new(4);

    println!("Server running on http://{}", bind_address);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                pool.execute(|| handle_connection(stream));
            }
            Err(e) => {
                eprintln!("Failed to establish a connection: {}", e);
                if e.kind() == ErrorKind::Interrupted {
                    continue;
                } else {
                    break;
                }
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader: BufReader<&TcpStream> = BufReader::new(&stream);
    let mut lines = buf_reader.lines();

    let request_line = match lines.next() {
        Some(Ok(line)) => line,
        Some(Err(e)) => {
            eprintln!("Could not read line: {}", e);
            return;
        }
        None => {
            eprintln!("Could not parse request line from buf reader");
            return;
        }
    };

    let (status_line, filename) = match request_line.as_str() {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "public/index.html"),
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "public/index.html")
        }
        _ => ("HTTP/1.1 404 NOT FOUND", "public/404.html"),
    };

    let contents: String = match fs::read_to_string(filename) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Could not read content of file {}: {}", filename, e);
            return;
        }
    };

    let response: String = format!(
        "{status_line}\r\nContent-Length: {}\r\n\r\n{}",
        contents.len(),
        contents
    );
    if let Some(route) = request_line.split(" ").nth(1) {
        println!("writing response for {:?}", route);
    }
    if let Err(e) = stream.write_all(response.as_bytes()) {
        eprintln!("Failed to write response to stream: {}", e);
    };
}
