# Contributing to rfb-encodings

Thank you for your interest in contributing to rfb-encodings! We welcome contributions from the community.

## Code of Conduct

This project adheres to a Code of Conduct that all contributors are expected to follow. Please read [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) before contributing.

## How to Contribute

### Reporting Bugs

If you find a bug, please create an issue with:
- A clear, descriptive title
- Steps to reproduce the issue
- Expected behavior vs actual behavior
- Your environment (OS, Rust version, rfb-encodings version)
- Any relevant logs or error messages
- Sample code or test case demonstrating the bug

### Suggesting Features

We welcome feature suggestions! Please create an issue with:
- A clear description of the feature
- Why you think it would be useful
- Any implementation ideas you have
- Whether it aligns with the RFB/VNC protocol specification

### Pull Requests

1. **Fork the repository** and create your branch from `main`
2. **Write code** following our style guidelines (see below)
3. **Add tests** for any new functionality
4. **Update documentation** if you've changed APIs
5. **Run tests** to ensure everything passes
6. **Submit a pull request** with a clear description

## Development Setup

### Prerequisites

- Rust 1.90 or later
- `libjpeg-turbo` (optional, for turbojpeg feature)

### Building

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/rfb-encodings.git
cd rfb-encodings

# Build
cargo build

# Run tests
cargo test

# Build with all features
cargo build --all-features
```

## Style Guidelines

### Code Style

- Follow standard Rust formatting: `cargo fmt`
- Pass clippy lints: `cargo clippy -- -D warnings`
- Write idiomatic Rust code
- Keep functions focused and reasonably sized
- Use descriptive variable names
- Avoid unsafe code unless absolutely necessary and well-documented

### Documentation

- Add doc comments (`///`) for all public items
- Include examples in doc comments where helpful
- Keep documentation up to date with code changes
- Write clear commit messages
- Reference RFC 6143 sections when implementing protocol features

### Commit Messages

Follow the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
type(scope): subject

body (optional)

footer (optional)
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvements
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

**Examples:**
```
feat(zrle): improve subencoding selection algorithm
fix(rre): prevent data loss in high-color images
docs(readme): add pixel format translation examples
perf(tight): optimize palette generation
```

## Testing

### Writing Tests

- Write unit tests for new functions
- Add integration tests for encoding/decoding correctness
- Test edge cases (empty images, single pixels, large images)
- Ensure tests are deterministic
- Use descriptive test names
- Test different pixel formats (8/16/24/32-bit)

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run with all features
cargo test --all-features

# Run without default features
cargo test --no-default-features
```

## Adding New Encodings

If you're adding a new VNC encoding:

1. Create a new file in `src/` (e.g., `src/newencoding.rs`)
2. Implement the `Encoding` trait
3. Follow RFC 6143 or relevant VNC extension specification
4. Add the encoding constant to `src/lib.rs`
5. Add comprehensive tests
6. Update README.md with the new encoding
7. Add entry to `get_encoder()` function
8. Document any special requirements or limitations

### Encoding Implementation Checklist

- [ ] Follows RFC 6143 or official specification
- [ ] Implements `Encoding` trait correctly
- [ ] Handles all pixel formats appropriately
- [ ] Includes compression/quality parameter handling
- [ ] Has unit tests for basic functionality
- [ ] Has integration tests with real pixel data
- [ ] Documented with examples
- [ ] Added to README.md
- [ ] Performance characteristics documented

## Performance Considerations

- Profile code changes for performance impact
- Avoid unnecessary allocations
- Use zero-copy techniques where possible
- Consider persistent stream compression state
- Document any performance trade-offs
- Benchmark critical code paths

## Pixel Format Translation

When working with pixel format translation:

- Test all supported pixel formats (8/16/24/32-bit)
- Verify big-endian and little-endian handling
- Test edge cases (odd bit depths, unusual shift values)
- Ensure color accuracy is maintained
- Document any precision loss

## Documentation

### API Documentation

Generate and review documentation:

```bash
cargo doc --open --all-features
```

Ensure all public items have:
- Summary description
- Parameter explanations
- Return value description
- Example code (where appropriate)
- Links to RFC 6143 sections
- Links to related items

### README Updates

If your changes affect:
- Features
- API
- Supported encodings
- Installation
- Usage examples

Please update README.md accordingly.

## Release Process

Maintainers follow this release process:

1. Update CHANGELOG.md
2. Bump version in Cargo.toml
3. Update version references in README.md
4. Create git tag: `git tag -a v0.x.x -m "Release v0.x.x"`
5. Push tag: `git push origin v0.x.x`
6. Publish to crates.io: `cargo publish`

## Getting Help

- Create an issue for questions
- Join discussions on GitHub
- Check existing issues and PRs
- Read the [TECHNICAL.md](TECHNICAL.md) for implementation details
- Reference [RFC 6143](https://www.rfc-editor.org/rfc/rfc6143.html) for protocol details

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 License.

## Recognition

Contributors are recognized in:
- Git commit history
- Release notes
- CHANGELOG.md

Thank you for contributing to rfb-encodings! 🦀
