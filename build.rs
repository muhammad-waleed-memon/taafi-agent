// Copyright 2026 Muhammad Waleed
// Licensed under the Apache License, Version 2.0
// Author: Muhammad Waleed

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/agent.proto")?;
    Ok(())
}
