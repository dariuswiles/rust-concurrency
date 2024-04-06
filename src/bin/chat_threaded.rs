/// A chat server that listens on a local IPv6 TCP port for incoming client connections and
/// broadcasts every line of input received to each currently connected client. A simple client
/// connection can be established on the the same machine by entering something like:
///     nc -Nv ::1 8080
///
/// This uses the concurrency provided by `std::thread` to handle each client's connection in a
/// separate OS thread. The child threads are detached from the parent thread, so the parent does
/// not need to wait for them to finish as part of program clean-up. A single thread is also
/// created to broadcast messages to clients.
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv6Addr, SocketAddrV6, TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

const LOCAL_ADDR_IPV6: Ipv6Addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1); // Represents [::1]
const LOCAL_PORT: u16 = 8080;

#[derive(Clone, Debug)]
struct Message {
    data: String,
}

fn main() {
    let time_at_start = Instant::now();
    println!("Starting at monotonic clock time: {:?}", time_at_start);

    let socket = SocketAddrV6::new(LOCAL_ADDR_IPV6, LOCAL_PORT, 0, 0);
    let listener = TcpListener::bind(socket).expect("Failed to bind to port {LOCAL_PORT}");

    let (broadcast_tx, broadcast_rx) = channel::<Message>();
    let user_streams_tx = Arc::new(Mutex::new(Vec::new()));

    // Spawn dedicated thread to broadcast messages to all TCP streams.
    let ust_cloned = user_streams_tx.clone();
    thread::spawn(move || {
        broadcast(broadcast_rx, ust_cloned);
    });

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!(
                    "{}ms: Connection established",
                    time_at_start.elapsed().as_millis()
                );

                let sender_cloned = broadcast_tx.clone();
                let stream_cloned = stream
                    .try_clone()
                    .expect("Failed to clone stream for handler");
                thread::spawn(move || {
                    handle_connection(stream_cloned, sender_cloned);
                });
                println!("Handler spawned");

                let mut a = user_streams_tx.lock().unwrap();
                a.push(stream);
                println!("Client registration complete");
            }
            Err(e) => {
                panic!("Incoming connection failed with error: {e:?}");
            }
        }

        println!("Control returned to main loop - waiting for more incoming connections");
    }
}

/// Continuously broadcasts `Messages` received on the given `broadcast_rx` `Receiver` to every
/// `TcpStream` in `user_streams_tx`. The latter is wrapped in an `Arc` and `Mutex` to allow the
/// vector of user streams to be updated by a different thread as new clients connect. If an attempt
/// to send data to a client user stream fails, the client is assumed to have disconnected and their
/// client user stream is removed from `user_streams_tx`.
///
/// The function loops continuously until an error occurs when trying to read from `broadcast_rx`.
fn broadcast(broadcast_rx: Receiver<Message>, user_streams_tx: Arc<Mutex<Vec<TcpStream>>>) {
    println!("Broadcaster started");
    loop {
        match broadcast_rx.recv() {
            Ok(message) => {
                println!("\tBroadcaster received message: {}", message.data);

                let mut good_senders = Vec::new();
                let mut streams = user_streams_tx.lock().unwrap();

                let response_bytes = ("Server responds: ".to_string() + &message.data).into_bytes();

                for mut stream in streams.drain(..) {
                    match stream.write_all(&response_bytes) {
                        Ok(()) => {
                            println!("\tSucceeded in broadcasting to a channel");
                            good_senders.push(stream);
                        }
                        Err(_) => {
                            println!("\tFailed to broadcast to a channel");
                        }
                    }
                }

                *streams = good_senders;
            }
            Err(e) => {
                println!(
                    "Broadcaster channel returned '{:?}', so Broadcaster exiting",
                    e
                );
                return;
            }
        }
    }
}

/// Continuously receives newline-delimited input from the `stream` passed, and sends it as a
/// `Message` to the given `sender` channel. This process is repeated until `stream` is closed or
/// an error occurs.
///
/// # Panics
///
/// Panics if an error occurs when sending to `sender` or when attempting to read data from
/// `stream`.
fn handle_connection(stream: TcpStream, sender: Sender<Message>) {
    let peer = stream
        .peer_addr()
        .expect("Failed to query details of the remote peer");
    println!("\tIncoming connection is from: {peer:?}");

    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    loop {
        match reader.read_line(&mut line) {
            Ok(0) => {
                // End of file
                println!("\t>>[End of data; closing connection]");
                return;
            }
            Ok(n) => {
                print!("\t>>[{n} chars] {line}"); // No need for newline as input contains one
                sender
                    .send(Message { data: line })
                    .expect("Failed to send incoming message to broadcaster");

                line = String::new();
            }
            Err(e) => {
                panic!("\tError while reading from received data:\n\t{e}");
            }
        }
    }
}
