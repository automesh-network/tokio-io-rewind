# tokio-io-rewind

`tokio-io-rewind` is a Rust crate providing a wrapper for types implementing the `AsyncRead` and `AsyncWrite` traits, allowing for the prepending of bytes to the read stream. This functionality is particularly useful in scenarios where data read from a stream needs to be re-examined or processed again, making it an ideal choice for networking, parsing, and proxying tasks where data manipulation is crucial.

## Features

- **Prepend Bytes**: Easily prepend bytes to the beginning of your read stream.
- **Transparent Async Operations**: Works seamlessly with async read and write operations, ensuring minimal overhead.
- **Flexibility**: Can be used with any type that implements `AsyncRead` and `AsyncWrite`, offering wide applicability.

## Usage

To use `tokio-io-rewind`, add it as a dependency in your `Cargo.toml`:

```toml
[dependencies]
tokio-io-rewind = "0.1"
```

### Basic Example

```rust
use tokio_io_rewind::Rewind;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use bytes::Bytes;

#[tokio::main]
async fn main() {
    let inner_stream = tokio::io::Cursor::new(b"world".to_vec());
    let mut rewind_stream = Rewind::new(inner_stream);
    
    // Prepend "hello " to the stream
    rewind_stream.rewind(Bytes::from_static(b"hello "));
    
    let mut buf = Vec::new();
    rewind_stream.read_to_end(&mut buf).await.unwrap();
    
    assert_eq!(buf, b"hello world");
}
```

## Advanced Usage

For more advanced use cases, such as pre-buffering data or handling complex async write operations, `tokio-io-rewind` can be configured with additional buffer states or integrated into custom async flows.

### Buffered Initialization

To initialize with a buffer, you can use:

```rust
let preloaded_buffer = Bytes::from_static(b"initial data");
let rewind_stream = Rewind::new_buffered(inner_stream, preloaded_buffer);
```

This preloads the stream with "initial data" that will be read before any subsequent data.

## License

[MIT License](LICENSE)