# Watcher

A library for subscribing to key-value data changes.

## Overview

Watch for changes in specific key ranges and get notified when data is updated or deleted. Useful for building reactive systems that need to respond to data changes.

## Features

- **Range-based watching** - Monitor key ranges instead of individual keys
- **Event filtering** - Choose to receive updates, deletes, or both
- **Multiple watchers** - Support concurrent watchers with different ranges and filters
- **Generic types** - Customize key/value types for any storage backend
- **Async streaming** - Built on tokio with non-blocking event delivery
- **Automatic cleanup** - Resources cleaned up when watchers are dropped
- **Efficient routing** - Fast event dispatch to interested watchers only

## Usage

```rust
use std::ops::Bound;
use watcher::{Dispatcher, EventFilter, TypeConfig};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy)]
struct Config;

impl TypeConfig for Config {
    type Key = String;
    type Value = String;
    type Response = (String, Option<String>, Option<String>);
    type Error = std::io::Error;

    fn new_initialize_response(key: Self::Key, value: Self::Value) -> Self::Response {
        (key, None, Some(value))
    }

    fn new_change_response(change: (Self::Key, Option<Self::Value>, Option<Self::Value>)) -> Self::Response {
        change
    }

    fn data_error(error: std::io::Error) -> Self::Error { error }
    fn update_watcher_metrics(_delta: i64) {}
    fn spawn<T>(fut: T) where T: std::future::Future + Send + 'static, T::Output: Send + 'static {
        tokio::spawn(fut);
    }
}

#[tokio::main]
async fn main() {
    let dispatcher = Dispatcher::<Config>::spawn();
    
    let (tx, mut rx) = mpsc::channel(10);
    let _watcher = dispatcher.add_watcher(
        (Bound::Included("a".to_string()), Bound::Excluded("z".to_string())),
        EventFilter::all(),
        tx
    ).await.unwrap();
    
    dispatcher.send_change(("key1".to_string(), None, Some("value1".to_string())));
    
    if let Some(Ok(event)) = rx.recv().await {
        println!("Received: {:?}", event);
    }
}
```

## Event Filters

- `EventFilter::all()` - Watch both updates and deletes
- `EventFilter::update()` - Watch only updates  
- `EventFilter::delete()` - Watch only deletes

## License

Apache-2.0
