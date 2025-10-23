# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue in rfb-encodings, please report it responsibly.

### How to Report

**Please do NOT create a public GitHub issue for security vulnerabilities.**

Instead, please report security issues by:

1. **Email**: Send details to dustin.mcafee@my.maryvillecollege.edu
2. **Subject**: Include "rfb-encodings Security" in the subject line
3. **Details**: Provide as much information as possible:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if any)

### What to Expect

- **Acknowledgment**: Within 48 hours
- **Initial Assessment**: Within 1 week
- **Status Updates**: Regular updates on progress
- **Resolution**: Security patches released as soon as possible
- **Credit**: You will be credited in the security advisory (unless you prefer to remain anonymous)

## Security Considerations

### Input Validation

rfb-encodings is designed to process potentially untrusted pixel data. When using this library:

- **Validate dimensions**: Ensure width/height values are reasonable before encoding
- **Check data length**: Verify pixel data length matches expected dimensions
- **Handle errors**: Always handle encoding errors gracefully

### Memory Safety

- This crate is written in safe Rust with minimal unsafe code
- All unsafe code is clearly documented and audited
- Memory allocations are bounded and validated
- No buffer overflows or use-after-free vulnerabilities

### Compression Attacks

When using compression-based encodings (Tight, ZRLE, etc.):

- Be aware of potential compression bombs
- Consider limiting maximum compressed data size
- Monitor memory usage when processing untrusted data

### Dependencies

We regularly audit and update dependencies:

- Use `cargo audit` to check for known vulnerabilities
- Keep dependencies up to date
- Review dependency changes before updating

## Best Practices

### When Using rfb-encodings

1. **Validate Input**
   ```rust
   // Check dimensions are reasonable
   if width > MAX_WIDTH || height > MAX_HEIGHT {
       return Err("Invalid dimensions");
   }

   // Verify data length
   let expected_len = width as usize * height as usize * 4;
   if data.len() != expected_len {
       return Err("Invalid data length");
   }
   ```

2. **Handle Errors**
   ```rust
   match encoder.encode(&data, width, height, quality, compression) {
       Ok(encoded) => process(encoded),
       Err(e) => {
           log::error!("Encoding failed: {}", e);
           // Handle gracefully
       }
   }
   ```

3. **Resource Limits**
   ```rust
   // Limit maximum rectangle size
   const MAX_RECT_SIZE: usize = 4096 * 4096;

   if (width as usize * height as usize) > MAX_RECT_SIZE {
       return Err("Rectangle too large");
   }
   ```

## Security Features

### Memory Safety

- ✅ No unsafe code in core encoding logic
- ✅ Bounds checking on all array access
- ✅ No manual memory management
- ✅ Rust's ownership system prevents use-after-free

### Input Validation

- ✅ Rectangle dimension validation
- ✅ Pixel data length verification
- ✅ Compression level bounds checking
- ✅ Quality parameter validation

### Error Handling

- ✅ All errors are typed and handled
- ✅ No panics on invalid input
- ✅ Graceful degradation

## Known Limitations

### Resource Exhaustion

- Large images may consume significant memory during encoding
- Consider implementing timeouts for encoding operations
- Monitor memory usage in production systems

### Compression Bombs

- Maliciously crafted data could produce large compressed output
- Applications should implement size limits on compressed data
- Consider using streaming compression where appropriate

## Security Audit History

| Date | Version | Auditor | Summary |
|------|---------|---------|---------|
| 2025-10-23 | 0.1.0 | Internal | Initial security review |

## Security Updates

Security updates will be released as patch versions (e.g., 0.1.1 → 0.1.2) and documented in:

- GitHub Security Advisories
- CHANGELOG.md
- Release notes

Subscribe to repository notifications to stay informed about security updates.

## Contact

For security-related questions (non-vulnerabilities):
- Open a GitHub Discussion
- Email: dustin.mcafee@my.maryvillecollege.edu

Thank you for helping keep rfb-encodings secure!
