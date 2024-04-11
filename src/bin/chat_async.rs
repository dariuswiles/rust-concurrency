/// A chat server that listens on a local IPv6 TCP port for incoming client connections and
/// broadcasts every line of input received to each currently connected client. A simple client
/// connection can be established on the the same machine by entering something like:
///     nc -Nv ::1 8080
///
/// This uses the cooperative multitasking provided by Rust's async/.await system in conjuction
/// with the async-std crate to handle each client's connection and the relaying of chat messages.
use async_std::channel::{self, Receiver, Sender};
use async_std::io::prelude::BufReadExt;
use async_std::io::{BufReader, WriteExt};
use async_std::net::{Ipv6Addr, SocketAddrV6, TcpListener, TcpStream};
use async_std::stream::StreamExt;
use async_std::sync::{Arc, Mutex};
use async_std::task;
use std::time::Instant;

const LOCAL_ADDR_IPV6: Ipv6Addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1); // Represents [::1]
const LOCAL_PORT: u16 = 8080;

type Message = String;

fn main() {
    let time_at_start = Instant::now();
    println!("Starting at monotonic clock time: {:?}", time_at_start);

    let accept_loop = async {
        let socket = SocketAddrV6::new(LOCAL_ADDR_IPV6, LOCAL_PORT, 0, 0);
        let listener = TcpListener::bind(socket)
            .await
            .expect("Failed to bind to port {LOCAL_PORT}");

        let mut incoming = listener.incoming();

        let (broadcast_tx, broadcast_rx) = channel::unbounded::<Message>();
        let user_streams_tx = Arc::new(Mutex::new(Vec::new()));

        // Spawn dedicated thread to broadcast messages to all TCP streams.
        let ust_cloned = user_streams_tx.clone();
        task::spawn(async {
            broadcast(broadcast_rx, ust_cloned).await;
        });

        while let Some(stream) = incoming.next().await {
            let stream = stream.unwrap();

            println!(
                "{}ms: Connection established",
                time_at_start.elapsed().as_millis()
            );

            let sender_cloned = broadcast_tx.clone();
            let stream_cloned = stream.clone();
            task::spawn(handle_connection(stream_cloned, sender_cloned));

            println!("Control returned to main loop - waiting for more incoming connections");

            let mut a = user_streams_tx.lock().await;
            a.push(stream);
            println!("Client registration complete");
        }
    };

    task::block_on(accept_loop);
}

/// Continuously broadcasts `Messages` received on the given `broadcast_rx` `Receiver` to every
/// `TcpStream` in `user_streams_tx`. The latter is wrapped in an `Arc` and `Mutex` to allow the
/// vector of user streams to be updated by a different thread as new clients connect. If an attempt
/// to send data to a client user stream fails, the client is assumed to have disconnected and their
/// client user stream is removed from `user_streams_tx`.
///
/// The function loops continuously until an error occurs when trying to read from `broadcast_rx`.
async fn broadcast(broadcast_rx: Receiver<Message>, user_streams_tx: Arc<Mutex<Vec<TcpStream>>>) {
    println!("Broadcaster started");
    loop {
        match broadcast_rx.recv().await {
            Ok(message) => {
                println!("\tBroadcaster received message: {}", message);

                let mut good_senders = Vec::new();
                let mut streams = user_streams_tx.lock().await;

                let response_bytes = message.into_bytes();

                for mut stream in streams.drain(..) {
                    match stream.write_all(&response_bytes).await {
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

/// First asks for the user's display name, then continuously receives newline-delimited input from
/// the `stream` passed, and sends it as a `Message` to the given `sender` channel. This process is
/// repeated until `stream` is closed or an error occurs.
///
/// # Panics
///
/// Panics if an error occurs when sending to `sender` or when attempting to read data from
/// `stream`.
async fn handle_connection(mut stream: TcpStream, sender: Sender<Message>) {
    let mut display_name = None;

    let peer = stream
        .peer_addr()
        .expect("Failed to query details of the remote peer");
    println!("\tIncoming connection is from: {peer:?}");

    stream
        .write_all(b"Enter your display name\n")
        .await
        .expect("Failed to send prompt for user to enter their display name");

    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    loop {
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // End of file
                println!("\t>>[End of data; closing connection]");
                return;
            }
            Ok(n) => {
                print!("\t>>[{n} chars] {line}"); // No need for newline as input contains one

                if display_name.is_none() {
                    display_name = Some((line.trim()).to_owned());

                    sender
                        .send(display_name.clone().unwrap().to_owned() + &" has entered the chat\n")
                        .await
                        .expect("Failed to send chat entry message to broadcaster");
                } else {
                    sender
                        .send(display_name.clone().unwrap().to_owned() + &": " + &line)
                        .await
                        .expect("Failed to send incoming message to broadcaster");
                }

                line = String::new();
            }
            Err(e) => {
                panic!("\tError while reading from received data:\n\t{e}");
            }
        }
    }
}
