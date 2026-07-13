#!/bin/bash
cd /tmp
cat > test.rs << 'RUST'
use std::io;
use serde::{Deserialize, Serialize};
