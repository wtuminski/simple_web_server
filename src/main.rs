use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};

use simple_web_server::ThreadPool;

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&stream);
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let (status_line, filepath) = match request_line.as_str() {
        "GET / HTTP/1.1" => ("HTTP/2.0 200 OK", "pages/hello.html"),
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(10));
            ("HTTP/2.0 200 OK", "pages/hello.html")
        }
        "GET /styles.css HTTP/1.1" => ("HTTP/2.0 200 OK", "pages/styles.css"),
        _ => ("HTTP/1.1 404 NOT FOUND", "pages/404.html"),
    };
    let contents = fs::read_to_string(filepath).unwrap();
    let length = contents.len();
    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();
}

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let pool = match ThreadPool::build(4) {
        Ok(pool) => pool,
        Err(reason) => {
            panic!("Pool creation error: {reason}");
        }
    };

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| handle_connection(stream));
    }
}
