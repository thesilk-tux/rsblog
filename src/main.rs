use std::{
    fs,
    io::{self, BufReader, ErrorKind, prelude::*},
    net::{TcpListener, TcpStream},
};

// TODO:
// - read all blog htmls and display them in iframe with route
//   - use filter to ship static files
// - add threadpool with unit test
// - integrate tera (template engine)

fn main() {
    let bind_address: &str = "127.0.0.1:8080";
    let listener: TcpListener = TcpListener::bind(bind_address).unwrap();

    println!("Server running on http://{}", bind_address);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(e) = handle_connection(stream) {
                    eprintln!("Connection handling error: {}", e);
                }
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

fn handle_connection(mut stream: TcpStream) -> io::Result<()> {
    let buf_reader: BufReader<&TcpStream> = BufReader::new(&stream);
    let mut lines = buf_reader.lines();

    let request_line = match lines.next() {
        Some(Ok(line)) => line,
        Some(Err(e)) => {
            eprintln!("Could not read line: {}", e);
            return Err(e);
        }
        None => {
            eprintln!("Could not parse request line from buf reader");
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Empty request"));
        }
    };

    let (status_line, filename) = if request_line == "GET / HTTP/1.1" {
        ("HTTP/1.1 200 OK", "public/index.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND", "public/404.html")
    };

    let contents: String = fs::read_to_string(filename).map_err(|e| {
        eprintln!("Could not read content of file {}: {}", filename, e);
        e
    })?;

    let response: String = format!(
        "{status_line}\r\nContent-Length: {}\r\n\r\n{}",
        contents.len(),
        contents
    );
    stream.write_all(response.as_bytes()).map_err(|e| {
        eprintln!("Failed to write response to stream: {}", e);
        e
    })?;
    Ok(())
}
