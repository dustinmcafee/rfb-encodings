# rfb-encodings Technical Documentation

This document provides detailed technical information about the rfb-encodings library architecture, encoding implementations, and design decisions.

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Encoding Implementations](#encoding-implementations)
- [Pixel Format System](#pixel-format-system)
- [Compression Strategy](#compression-strategy)
- [Performance Characteristics](#performance-characteristics)
- [Implementation Details](#implementation-details)

## Architecture Overview

### Design Philosophy

rfb-encodings is designed as a standalone, reusable library for RFB protocol encoding. Key principles:

1. **RFC 6143 Compliance** - Strict adherence to the RFB protocol specification
2. **Memory Safety** - Pure Rust with minimal unsafe code
3. **Zero Dependencies** (core) - Only essential dependencies: bytes, flate2, png
4. **Modularity** - Each encoding is independent and pluggable
5. **Performance** - Zero-copy where possible, efficient algorithms

### Module Structure

```
rfb-encodings/
├── lib.rs              # Public API, trait definitions, constants
├── common.rs           # Shared utilities (subrect finding, color analysis)
├── translate.rs        # Pixel format translation
├── raw.rs              # Raw encoding (baseline)
├── rre.rs              # Rise-and-Run-length Encoding
├── corre.rs            # Compact RRE
├── hextile.rs          # Tile-based encoding
├── tight.rs            # Advanced multi-mode encoding
├── tightpng.rs         # PNG-based encoding
├── zlib.rs             # Zlib-compressed raw
├── zlibhex.rs          # Zlib-compressed Hextile
├── zrle.rs             # Zlib Run-Length Encoding
└── zywrle.rs           # Wavelet-based compression
```

### Core Traits

#### `Encoding` Trait

```rust
pub trait Encoding {
    fn encode(
        &self,
        data: &[u8],        // RGBA pixel data
        width: u16,
        height: u16,
        quality: u8,        // 0-9, for lossy encodings
        compression: u8,    // 0-9, for zlib-based encodings
    ) -> BytesMut;
}
```

All encoders implement this trait for consistency.

#### `TightStreamCompressor` Trait

```rust
pub trait TightStreamCompressor {
    fn compress_tight_stream(
        &mut self,
        stream_id: u8,      // 0=full-color, 1=mono, 2=indexed
        level: u8,
        input: &[u8],
    ) -> Result<Vec<u8>, String>;
}
```

Allows VNC servers to maintain persistent compression state across frames.

## Encoding Implementations

### Raw Encoding (Type 0)

**Purpose:** Baseline encoding, no compression
**Wire Format:** Direct pixel data in client's pixel format

```rust
// Simply converts RGBA to client format
for chunk in data.chunks_exact(4) {
    buf.put_u8(chunk[0]); // R
    buf.put_u8(chunk[1]); // G
    buf.put_u8(chunk[2]); // B
    buf.put_u8(0);         // Padding
}
```

**Performance:**
- Encoding: O(n) - linear in pixel count
- Network: Highest bandwidth usage
- CPU: Minimal processing

**Use Cases:**
- Fallback when all else fails
- Very small rectangles where compression overhead exceeds benefit
- Testing and debugging

### RRE Encoding (Type 2)

**Purpose:** Encode large solid-color regions efficiently
**Wire Format:** `[nSubrects(u32)][bgColor(32-bit)][subrect1]...[subrectN]`

Each subrectangle: `[color(32-bit)][x(u16)][y(u16)][w(u16)][h(u16)]`

**Algorithm:**
1. Find most common color (background)
2. Find all rectangular regions of other colors
3. Encode as background + list of colored rectangles

**Critical Fix (v0.1.0):**
```rust
// OLD (buggy): Would discard data if inefficient
if encoded_size >= raw_size {
    return buf_with_only_background(); // DATA LOSS!
}

// NEW (correct): Always encode all pixels
let subrects = find_subrects(&pixels, width, height, bg_color);
// Encode all subrects, even if inefficient
```

**Performance:**
- Best case: O(n) for solid color or few colors
- Worst case: O(n²) for complex images (many subrects)
- Network: Excellent for simple graphics, poor for photos

**Use Cases:**
- Screenshots with large solid areas
- Desktop backgrounds
- UI elements with flat colors

### CoRRE Encoding (Type 4)

**Purpose:** Like RRE but optimized for small rectangles
**Wire Format:** `[bgColor(32-bit)][nSubrects(u8)][subrect1]...[subrectN]`

Each subrectangle: `[color(32-bit)][x(u8)][y(u8)][w(u8)][h(u8)]`

**Key Differences from RRE:**
- Uses 8-bit coordinates (max 255×255 tiles)
- Subrect count before background (different order)
- More compact for small regions

**Performance:**
- 50% smaller headers than RRE for small rectangles
- Limited to 255×255 pixel tiles
- Tile splitting for larger regions

### Hextile Encoding (Type 5)

**Purpose:** Tile-based encoding with multiple sub-encodings
**Tile Size:** 16×16 pixels

**Sub-encoding Types:**
1. **Raw** (0x01) - Uncompressed tile data
2. **Background Specified** (0x02) - New background color
3. **Foreground Specified** (0x04) - New foreground color
4. **Any Subrects** (0x08) - Contains subrectangles
5. **Subrects Colored** (0x10) - Each subrect has own color

**Algorithm per Tile:**
```rust
if tile_is_solid {
    encode_as_background_specified();
} else if tile_has_few_colors {
    encode_as_subrects();
} else {
    encode_as_raw();
}
```

**Performance:**
- Best case: O(1) per tile for solid colors
- Worst case: O(n) per tile for complex patterns
- Network: Good for mixed content (some solid, some detailed)

**Use Cases:**
- Desktop screenshots with mixed content
- Incremental updates (tiles can be cached)
- Balance between compression and CPU

### Tight Encoding (Type 7)

**Purpose:** Advanced multi-mode encoding with best compression
**Modes:** 5 different encoding strategies

#### Mode Selection Algorithm

```rust
fn select_tight_mode(colors: usize, is_gradient: bool, quality: u8) -> Mode {
    if colors == 1 {
        Mode::Solid        // 1 color: encode as single pixel
    } else if colors == 2 {
        Mode::Mono         // 2 colors: 1-bit bitmap with palette
    } else if colors <= 16 {
        Mode::Indexed      // 3-16 colors: indexed with palette
    } else if quality < 5 && !is_gradient {
        Mode::JPEG         // Many colors, lossy allowed: JPEG
    } else {
        Mode::FullColor    // Many colors, need lossless: zlib
    }
}
```

#### Mode 1: Solid Fill

```
Wire: [0x08][color (3 or 4 bytes)]
```

Most efficient mode, single color for entire rectangle.

#### Mode 2: Mono Rectangle

```
Wire: [0x00|StreamID][palette(2 colors)][zlib(bitmap)]
```

Two colors encoded as 1-bit-per-pixel bitmap compressed with zlib.

**Bitmap Encoding:**
```rust
// Pack 8 pixels into 1 byte
for y in rows {
    let mut byte = 0u8;
    for x in 0..8 {
        byte = (byte << 1) | (if pixel == fg_color { 1 } else { 0 });
    }
    bitmap.push(byte);
}
```

#### Mode 3: Indexed Palette

```
Wire: [0x80|palette_size-1][palette(3-16 colors)][zlib(indices)]
```

Up to 16 colors with each pixel represented as 1-4 bit index.

**Index Packing:**
- 2 colors: 1 bit/pixel (8 pixels/byte)
- 3-4 colors: 2 bits/pixel (4 pixels/byte)
- 5-16 colors: 4 bits/pixel (2 pixels/byte)

#### Mode 4: Full-Color Zlib

```
Wire: [0x00|StreamID][zlib(pixel_data)]
```

Lossless compression of RGB data using zlib.

#### Mode 5: JPEG

```
Wire: [0x90][length(compact)][jpeg_data]
```

Lossy JPEG compression for photographic content.

**Quality Mapping:**
```rust
const TIGHT_QUALITY_TO_JPEG: &[u8] = &[
    5,   // quality 0 -> JPEG 5%
    10,  // quality 1 -> JPEG 10%
    15,  // quality 2 -> JPEG 15%
    25,  // quality 3 -> JPEG 25%
    37,  // quality 4 -> JPEG 37%
    50,  // quality 5 -> JPEG 50%
    60,  // quality 6 -> JPEG 60%
    70,  // quality 7 -> JPEG 70%
    75,  // quality 8 -> JPEG 75%
    80,  // quality 9 -> JPEG 80%
];
```

### Persistent Compression Streams

Tight, ZRLE, Zlib, and ZlibHex use persistent compression streams.

**Why Persistent Streams?**
- Maintains dictionary from previous compressions
- Significantly better compression ratios for similar data
- Required by RFC 6143 for Tight encoding

**Stream IDs (Tight):**
- Stream 0: Full-color data
- Stream 1: Mono rectangles
- Stream 2: Indexed rectangles
- Stream 3: (Reserved)

**Implementation:**
```rust
pub struct SimpleTightCompressor {
    streams: [Option<flate2::Compress>; 4],
    level: u8,
}

impl TightStreamCompressor for SimpleTightCompressor {
    fn compress_tight_stream(&mut self, stream_id: u8, level: u8, input: &[u8])
        -> Result<Vec<u8>, String>
    {
        // Get or create persistent stream
        let stream = &mut self.streams[stream_id];

        // Use FlushCompress::Sync to maintain dictionary
        stream.compress(input, &mut output, FlushCompress::Sync)?;
    }
}
```

### TightPng Encoding (Type -260)

**Purpose:** PNG-based encoding for browser clients (noVNC)
**Wire Format:** `[0x0A][length][PNG data]`

**Advantages:**
- Hardware decoding in browsers
- No zlib decompression needed in JavaScript
- Lossless compression

**Implementation:**
```rust
let mut encoder = png::Encoder::new(buf, width, height);
encoder.set_color(png::ColorType::Rgb);
encoder.set_compression(png::Compression::Best);

// Convert RGBA -> RGB
let rgb_data: Vec<u8> = data.chunks_exact(4)
    .flat_map(|chunk| [chunk[0], chunk[1], chunk[2]])
    .collect();

writer.write_image_data(&rgb_data)?;
```

### ZRLE Encoding (Type 16)

**Purpose:** Zlib-compressed run-length encoding
**Tile Size:** 64×64 pixels

**Sub-encoding Types:**
- 0: Raw pixels
- 1: Solid color
- 2-16: Palette RLE (2-16 colors)
- 17-127: Packed palette (17-127 colors)
- 128: Plain RLE
- 129: Reuse palette
- 130-255: Primed palette RLE

**Algorithm:**
```rust
fn encode_zrle_tile(tile: &[u32]) -> Vec<u8> {
    let colors = count_unique_colors(tile);

    if colors == 1 {
        encode_solid(tile[0])
    } else if colors <= 16 {
        encode_palette_rle(tile, colors)
    } else if has_long_runs(tile) {
        encode_plain_rle(tile)
    } else {
        encode_raw(tile)
    }
}
```

**RLE Format:**
```
Run of color C with length N:
- If N = 1: Just the color index
- If N > 1: color_index + (N-1) as separate byte
```

### ZYWRLE Encoding (Type 17)

**Purpose:** Wavelet-based lossy compression
**Based on:** ZRLE with wavelet transform preprocessing

**Wavelet Levels:**
- Level 0: No filtering (lossless)
- Level 1: 2×2 wavelet filter
- Level 2: 4×4 wavelet filter
- Level 3: 8×8 wavelet filter

**Quality to Level Mapping:**
```rust
fn zywrle_level_from_quality(quality: u8) -> u8 {
    match quality {
        0..=1 => 3,  // Maximum filtering
        2..=4 => 2,  // Medium filtering
        5..=7 => 1,  // Light filtering
        _     => 0,  // No filtering (lossless)
    }
}
```

**Wavelet Transform:**
Uses 2D Cohen-Daubechies-Feauveau 9/7 wavelet for natural images.

## Pixel Format System

### PixelFormat Structure

```rust
pub struct PixelFormat {
    pub bits_per_pixel: u8,   // 8, 16, 24, or 32
    pub depth: u8,             // Color depth (bits used)
    pub big_endian_flag: u8,   // 0=LE, 1=BE
    pub true_colour_flag: u8,  // 1=RGB, 0=colormap
    pub red_max: u16,          // Max red value
    pub green_max: u16,        // Max green value
    pub blue_max: u16,         // Max blue value
    pub red_shift: u8,         // Red bit position
    pub green_shift: u8,       // Green bit position
    pub blue_shift: u8,        // Blue bit position
}
```

### Common Pixel Formats

#### RGBA32 (Server Format)
```rust
PixelFormat {
    bits_per_pixel: 32,
    depth: 24,
    big_endian_flag: 0,
    true_colour_flag: 1,
    red_max: 255,
    green_max: 255,
    blue_max: 255,
    red_shift: 0,    // R in bits 0-7
    green_shift: 8,  // G in bits 8-15
    blue_shift: 16,  // B in bits 16-23
}
```

#### RGB565 (Common Mobile)
```rust
PixelFormat {
    bits_per_pixel: 16,
    depth: 16,
    big_endian_flag: 0,
    true_colour_flag: 1,
    red_max: 31,     // 5 bits
    green_max: 63,   // 6 bits
    blue_max: 31,    // 5 bits
    red_shift: 11,
    green_shift: 5,
    blue_shift: 0,
}
```

#### BGR233 (Low Bandwidth)
```rust
PixelFormat {
    bits_per_pixel: 8,
    depth: 8,
    big_endian_flag: 0,
    true_colour_flag: 1,
    red_max: 7,      // 3 bits
    green_max: 7,    // 3 bits
    blue_max: 3,     // 2 bits
    red_shift: 0,
    green_shift: 3,
    blue_shift: 6,
}
```

### Pixel Translation

The `translate_pixels()` function converts between any two pixel formats:

```rust
pub fn translate_pixels(
    src: &[u8],
    server_format: &PixelFormat,
    client_format: &PixelFormat,
) -> BytesMut {
    // Extract RGB components from server format
    let (r, g, b) = extract_rgb(pixel, server_format);

    // Scale to client's bit depth
    let r_scaled = (r * client_format.red_max) / server_format.red_max;
    let g_scaled = (g * client_format.green_max) / server_format.green_max;
    let b_scaled = (b * client_format.blue_max) / server_format.blue_max;

    // Pack into client format
    let pixel = (r_scaled << client_format.red_shift)
              | (g_scaled << client_format.green_shift)
              | (b_scaled << client_format.blue_shift);

    // Write in client's endianness
    write_pixel(pixel, client_format);
}
```

## Compression Strategy

### Zlib Configuration

**Compression Levels:**
- 0: No compression (fastest)
- 1-3: Fast compression
- 4-6: Balanced (default range)
- 7-9: Best compression (slowest)

**Flush Modes:**
- `FlushCompress::Sync` - Maintain dictionary, partial flush
- `FlushCompress::Full` - Flush but maintain dictionary
- `FlushCompress::Finish` - Final flush, destroy dictionary

**rfb-encodings uses `Sync`** to maintain dictionary state.

### JPEG Configuration

**With TurboJPEG (feature):**
- Uses libjpeg-turbo via FFI
- Hardware acceleration (SIMD)
- 2-6× faster than software JPEG

**Without TurboJPEG:**
- Falls back to full-color zlib encoding
- Ensures compatibility on all platforms

## Performance Characteristics

### Encoding Speed (relative)

| Encoding | CPU Usage | Encoding Time | Compression Ratio |
|----------|-----------|---------------|-------------------|
| Raw      | Minimal   | Fastest       | 1:1 (none)        |
| RRE      | Low       | Fast          | Good (simple)     |
| Hextile  | Medium    | Medium        | Good              |
| Tight    | High      | Slow          | Excellent         |
| ZRLE     | Medium-High | Medium-Slow | Very Good         |

### Memory Usage

All encodings use `BytesMut` for efficient buffer management:

```rust
// Pre-allocate based on worst case
let capacity = width * height * bytes_per_pixel;
let mut buf = BytesMut::with_capacity(capacity);
```

### Optimization Techniques

1. **Zero-Copy**
   ```rust
   // Direct slice access, no copying
   for chunk in data.chunks_exact(4) {
       process(chunk);
   }
   ```

2. **Buffer Reuse**
   ```rust
   // Reuse compression buffers
   stream.compress(input, &mut reused_buffer, Flush::Sync);
   ```

3. **Early Exit**
   ```rust
   // Skip processing if result is obvious
   if pixels.iter().all(|&p| p == first_pixel) {
       return encode_solid(first_pixel);
   }
   ```

4. **Tile-Based Processing**
   ```rust
   // Process in tiles to improve cache locality
   for tile in tiles(64, 64) {
       encode_tile(tile);
   }
   ```

## Implementation Details

### Error Handling

All encoding errors are handled gracefully:

```rust
// Never panic on invalid input
pub fn encode(&self, data: &[u8], width: u16, height: u16) -> BytesMut {
    // Validate inputs
    let expected_len = (width as usize) * (height as usize) * 4;
    if data.len() != expected_len {
        // Return empty buffer or fallback encoding
        return BytesMut::new();
    }

    // Safe processing
    process_data(data, width, height)
}
```

### Thread Safety

- All encoders are `Send` and `Sync`
- No shared mutable state
- Each encoding operation is independent
- `TightStreamCompressor` requires `&mut self` for stream state

### Testing Strategy

1. **Unit Tests** - Individual function correctness
2. **Property Tests** - Invariants hold for all inputs
3. **Integration Tests** - End-to-end encoding
4. **Fuzz Testing** - Random input handling
5. **Benchmark Tests** - Performance regression

## References

- [RFC 6143](https://www.rfc-editor.org/rfc/rfc6143.html) - The RFB Protocol
- [Tight Encoding Specification](https://github.com/rfbproto/rfbproto/blob/master/rfbproto.rst#tight-encoding)
- [ZRLE Specification](https://github.com/rfbproto/rfbproto/blob/master/rfbproto.rst#zrle-encoding)

## Future Work

### Potential Optimizations

1. **SIMD for Pixel Translation**
   - Use packed SIMD for batch pixel conversion
   - Could provide 4-8× speedup

2. **Parallel Tile Encoding**
   - Encode tiles in parallel using rayon
   - Beneficial for large images

3. **Adaptive Encoding Selection**
   - Dynamically choose best encoding based on content
   - Machine learning for optimal mode selection

4. **Hardware Acceleration**
   - GPU-accelerated JPEG encoding
   - Hardware zlib compression where available

### Decoder Support

Future versions may include decoders for VNC client applications.

---

**Last Updated:** 2025-10-23
**Version:** 0.1.0
