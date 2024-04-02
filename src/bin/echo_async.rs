/// A server that listens on a local IPv6 TCP port for incoming connections and echoes each line of
/// input from a client back to that client. A simple client connection can be established on the
/// the same machine by entering something like:
///     nc -Nv ::1 8080
///
/// This code uses Rust's async/.await functionality to allow multiple clients to connect and have
/// their input echoed seemingly in parallel.
use async_std::net::{Ipv6Addr, SocketAddrV6, TcpListener, TcpStream};
use async_std::io::{prelude::BufReadExt, BufReader, WriteExt, };
use async_std::stream::StreamExt;
use async_std::task;
use std::time::Instant;


const LOCAL_ADDR_IPV6: Ipv6Addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1); // Represents [::1]
const LOCAL_PORT: u16 = 8080;

fn main() {
    let time_at_start = Instant::now();
    println!("Starting at monotonic clock time: {:?}", time_at_start);

    let accept_loop = async {
        let socket = SocketAddrV6::new(LOCAL_ADDR_IPV6, LOCAL_PORT, 0, 0);
        let listener = TcpListener::bind(socket)
            .await
            .expect("Failed to bind to port {LOCAL_PORT}");

        let mut incoming = listener.incoming();

        while let Some(stream) = incoming.next().await {
            let stream = stream.unwrap();

            println!(
                "{}ms: Connection established",
                time_at_start.elapsed().as_millis()
            );
            task::spawn(handle_connection(stream));

            println!("Control returned to main loop - waiting for more incoming connections");
        }
    };

    task::block_on(accept_loop);
}

/// Receives newline-delimited input from `stream`, and sends the same data back on the same stream.
async fn handle_connection(mut stream: TcpStream) {
    let peer = stream
        .peer_addr()
        .expect("Failed to query details of the remote peer");
    println!("\tIncoming connection is from: {peer:?}");

    let mut reader = BufReader::new(stream.clone());
    let mut line = String::new();

    loop {
        match reader.read_line(&mut line).await {
            Ok(0) => { // End of file
                println!("\t>>[End of data; closing connection]");
                return;
            }
            Ok(n) => {
                print!("\t>>[{n} chars] {line}"); // No need for newline as input contains one
                let response_bytes = ("Server responds: ".to_string() + &line).into_bytes();
                stream
                    .write_all(&response_bytes)
                    .await
                    .expect("Error occurred sending client response");
                line.clear();
            }
            Err(e) => {
                panic!("\tError while reading from received data:\n\t{e}");
            }
        }
    }
}
