# zeebe-rs

[![CI](https://github.com/Bentebent/zeebe-rs/actions/workflows/ci.yml/badge.svg?event=pull_request)](https://github.com/Bentebent/zeebe-rs/actions/workflows/ci.yml)

A Rust client and worker implementation for interacting with [Zeebe](https://camunda.com/platform/zeebe/) built using [Tonic](https://github.com/hyperium/tonic) and [Tokio](https://github.com/tokio-rs/tokio).

## Usage

## Development

Built using:

- rustc 1.83.0
- cargo 1.85.0-nightly (4c39aaff6 2024-11-25) (fmt and clippy only)
- [protoc v29.2](https://github.com/protocolbuffers/protobuf/releases/tag/v29.2)

## TODO

- [ ] Zeebe worker
- [ ] Unit tests
- [ ] Continuous delivery to crates.io
- [ ] Automated protobuf updates
- [ ] Examples
- [ ] Documentation

## License

You can choose between [MIT License](https://opensource.org/licenses/MIT) or [Apache License 2.0](http://www.apache.org/licenses/LICENSE-2.0).

### MIT License

Copyright (c) 2025 Tim Henriksson

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice (including the next paragraph) shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

### Apache License 2.0

Copyright 2025 Tim Henriksson

Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at

http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
