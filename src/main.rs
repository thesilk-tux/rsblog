use std::{
    collections::HashMap,
    fs,
    io::{BufReader, ErrorKind, prelude::*},
    net::{TcpListener, TcpStream},
    path::Path,
    sync::Arc,
};

use rsblog::ThreadPool;

// TODO:
// - read all blog htmls and display them in iframe with route
//   - use filter to ship static files
// - integrate tera (template engine)

fn main() {
    let routes = match filenames_to_paths("public/pages") {
        Ok(mut map) => {
            map.insert(
                "GET / HTTP/1.1".to_string(),
                "public/index.html".to_string(),
            );
            Arc::new(map)
        }
        Err(e) => {
            eprintln!("Could not find given Directory: {}", e);
            return;
        }
    };

    for (filename, path) in &*routes {
        println!("{}: {}", filename, path);
    }

    let bind_address: &str = "127.0.0.1:8080";
    let listener: TcpListener = TcpListener::bind(bind_address).unwrap();
    let pool = ThreadPool::new(4);

    println!("Server running on http://{}", bind_address);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let routes = Arc::clone(&routes); // Clone Arc for each thread
                pool.execute(|| handle_connection(stream, routes));
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

fn handle_connection(mut stream: TcpStream, routes: Arc<HashMap<String, String>>) {
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

    let filename = routes
        .get(&request_line)
        .map(|path| path.as_str())
        .unwrap_or("public/404.html");

    let status_line = if routes.contains_key(&request_line) {
        "HTTP/1.1 200 OK"
    } else {
        "HTTP/1.1 404 NOT FOUND"
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

/// Reads all filenames in the given directory and returns a HashMap
/// where the key is the filename and the value is the full path.
pub fn filenames_to_paths<P: AsRef<Path>>(dir: P) -> std::io::Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            let key = format!(
                "GET /{} HTTP/1.1",
                filename.to_string().replace(".html", "")
            );
            map.insert(key, path.display().to_string());
        }
    }
    Ok(map)
}
