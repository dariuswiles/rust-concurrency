# Experiments with Rust Multithreading and Multitasking

This repository contains basic programs to experiment with and compare a couple of Rust concurrency mechanisms, namely:

* [std::thread](https://doc.rust-lang.org/std/thread/index.html) in Rust's standard library
* Rusts __async__ and __.await__ keywords in conjuncion with the [async-std](https://docs.rs/async-std/latest/async_std/) crate

The programs have minimal error checking and are unlikely to be idiomatic Rust, so only use the code for experimentation.


## Programs
### Echo Server

A server that accepts multiple TCP IPv6 network connections and echoes back each line of text that a client sends to the client that sent it. A network connection is held open until the client disconnects, during which time the client can send multiple lines of text and receive a response after each one. It is a more interesting problem to solve that a web server as 

* __echo_simple__. A single-threaded program that only handles a single connection at a time. If multiple clients attempt to connect simultaneously, all but the first are queued and will not receive responses until the client that connected first closes its connection. This program is intended purely as a baseline to compare the concurrent variants against.
* __echo_threaded__. Uses [std::thread](https://doc.rust-lang.org/std/thread/index.html)'s multithreading to respond to multiple connections concurrently.
* __echo_async__. Uses async/.await in conjunction with the [async-std](https://docs.rs/async-std/latest/async_std/) crate's asynchronous versions of standard library functionality to respond to multiple connections concurrently using cooperative multitasking.

### Chat Server

A server that accepts multiple TCP IPv6 network connections and broadcasts each line of input received from any client to all connected clients. Each client is first asked for a display name that is prepended to every line broadcast .

* __chat_threaded__. Uses [std::thread](https://doc.rust-lang.org/std/thread/index.html)'s multithreading to create a dedicated thread for each client connection and one to broadcast incoming input to all threads.
* __chat_async__. Uses async/.await in conjunction with the [async-std](https://docs.rs/async-std/latest/async_std/) crate's asynchronous versions of standard library functionality to create a dedicated task for each client connection and one to broadcast incoming input to all other tasks.

## License

This repository is released under The Unlicense. See [LICENSE](LICENSE) for the license text, or https://unlicense.org/ for more details.
