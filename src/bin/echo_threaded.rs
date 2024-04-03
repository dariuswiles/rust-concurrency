/// A server that listens on a local IPv6 TCP port for incoming connections and echoes each line of
/// input from a client back to that client. A simple client connection can be established on the
/// the same machine by entering something like:
///     nc -Nv ::1 8080
///
/// This uses the concurrency provided by `std::thread` to handle each client's connection in a
/// separate OS thread. The child threads are detached from the parent thread, so the parent does
/// not need to wait for them to finish as part of program clean-up. OS threads are a bit overkill
/// for this simple task, but required minimal changes to the code to implement.
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv6Addr, SocketAddrV6, TcpListener, TcpStream};
use std::thread;
use std::time::Instant; // NEW for threading

const LOCAL_ADDR_IPV6: Ipv6Addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1); // Represents [::1]
const LOCAL_PORT: u16 = 8080;

fn main() {
    let time_at_start = Instant::now();
    println!("Starting at monotonic clock time: {:?}", time_at_start);

    let socket = SocketAddrV6::new(LOCAL_ADDR_IPV6, LOCAL_PORT, 0, 0);
    let listener = TcpListener::bind(socket).expect("Failed to bind to port {LOCAL_PORT}");

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!(
                    "{}ms: Connection established",
                    time_at_start.elapsed().as_millis()
                );

                #[rustfmt::skip]
                thread::spawn(move || { // NEW for threading
                    handle_connection(&mut stream);
                }); // NEW for threading
            }
            Err(e) => {
                panic!("Incoming connection failed with error: {e:?}",);
            }
        }

        println!("Control returned to main loop - waiting for more incoming connections");
    }
}

/// Receives newline-delimited input from `stream`, and sends the same data back on the same stream.
fn handle_connection(stream: &mut TcpStream) {
    let peer = stream
        .peer_addr()
        .expect("Failed to query details of the remote peer");
    println!("\tIncoming connection is from: {peer:?}");

    let mut reader = BufReader::new(stream.try_clone().expect("Failed to clone network stream"));
    let mut line = String::new();

    loop {
        match reader.read_line(&mut line) {
            Ok(0) => {
                // End of file
                println!("\t>>[End of data; closing connection]");
                return;
            }
            Ok(n) => {
                // Ok(n) if n > 0 => {
                print!("\t>>[{n} chars] {line}"); // No need for newline as input contains one
                let response_bytes = ("Server responds: ".to_string() + &line).into_bytes();
                stream
                    .write_all(&response_bytes)
                    .expect("Error occurred sending client response");
                line.clear();
            }
            Err(e) => {
                panic!("\tError while reading from received data:\n\t{e}");
            }
        }
    }
}
