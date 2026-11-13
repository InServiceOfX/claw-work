# fastcli

A blazing-fast CLI framework for building developer tools in Rust.

![Demo](demo.gif)

## Why fastcli?

Tired of writing the same argument parsing boilerplate? fastcli gives you:

- Zero-config command parsing
- Automatic help generation  
- Built-in shell completions
- Type-safe arguments

## Quick Start

```bash
cargo add fastcli
```

```rust
use fastcli::prelude::*;

#[derive(Command)]
struct MyApp {
    #[arg(short, long)]
    name: String,
}

fn main() {
    let app = MyApp::parse();
    println!("Hello, {}!", app.name);
}
```

## Installation

1. Add fastcli to your Cargo.toml
2. Derive the Command trait
3. Run your app

## Features

- **Fast**: Zero runtime overhead
- **Simple**: Derive macros do the work
- **Flexible**: Customize everything

## License

MIT
