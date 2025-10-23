// Copyright 2025 Dustin McAfee
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! RFB (Remote Framebuffer) protocol encoding implementations.
//!
//! This crate provides encoding implementations for the VNC/RFB protocol,
//! including all standard encodings: Raw, RRE, `CoRRE`, Hextile, Tight, `TightPng`,
//! Zlib, `ZlibHex`, ZRLE, and ZYWRLE.

#![deny(missing_docs)]
#![warn(clippy::pedantic)]

use bytes::{Buf, BufMut, BytesMut};
use std::io;

// Encoding modules
pub mod common;
pub mod corre;
pub mod hextile;
pub mod jpeg;
pub mod raw;
pub mod rre;
pub mod tight;
pub mod tightpng;
pub mod translate;
pub mod zlib;
pub mod zlibhex;
pub mod zrle;
pub mod zywrle;

// Encoding type constants (from RFC 6143)

/// Encoding type: Raw pixel data.
pub const ENCODING_RAW: i32 = 0;

/// Encoding type: Copy Rectangle.
pub const ENCODING_COPYRECT: i32 = 1;

/// Encoding type: Rise-and-Run-length Encoding.
pub const ENCODING_RRE: i32 = 2;

/// Encoding type: Compact RRE.
pub const ENCODING_CORRE: i32 = 4;

/// Encoding type: Hextile.
pub const ENCODING_HEXTILE: i32 = 5;

/// Encoding type: Zlib compressed.
pub const ENCODING_ZLIB: i32 = 6;

/// Encoding type: Tight.
pub const ENCODING_TIGHT: i32 = 7;

/// Encoding type: `ZlibHex`.
pub const ENCODING_ZLIBHEX: i32 = 8;

/// Encoding type: Zlib compressed TRLE.
pub const ENCODING_ZRLE: i32 = 16;

/// Encoding type: ZYWRLE (Zlib+Wavelet+Run-Length Encoding).
pub const ENCODING_ZYWRLE: i32 = 17;

/// Encoding type: `TightPng`.
pub const ENCODING_TIGHTPNG: i32 = -260;

// Re-export common types
pub use common::*;
pub use corre::CorRreEncoding;
pub use hextile::HextileEncoding;
pub use raw::RawEncoding;
pub use rre::RreEncoding;
pub use tight::TightEncoding;
pub use tightpng::TightPngEncoding;
pub use zlib::encode_zlib_persistent;
pub use zlibhex::encode_zlibhex_persistent;
pub use zrle::encode_zrle_persistent;
pub use zywrle::zywrle_analyze;

// Hextile subencoding flags

/// Hextile: Raw pixel data for this tile.
pub const HEXTILE_RAW: u8 = 1 << 0;

/// Hextile: Background color is specified.
pub const HEXTILE_BACKGROUND_SPECIFIED: u8 = 1 << 1;

/// Hextile: Foreground color is specified.
pub const HEXTILE_FOREGROUND_SPECIFIED: u8 = 1 << 2;

/// Hextile: Tile contains subrectangles.
pub const HEXTILE_ANY_SUBRECTS: u8 = 1 << 3;

/// Hextile: Subrectangles are colored (not monochrome).
pub const HEXTILE_SUBRECTS_COLOURED: u8 = 1 << 4;

// Tight subencoding types

/// Tight/TightPng: PNG compression subencoding.
pub const TIGHT_PNG: u8 = 0x0A;

/// Represents the pixel format used in RFB protocol.
///
/// This struct defines how pixel data is interpreted, including color depth,
/// endianness, and RGB component details.
#[derive(Debug, Clone)]
pub struct PixelFormat {
    /// Number of bits per pixel.
    pub bits_per_pixel: u8,
    /// Depth of the pixel in bits.
    pub depth: u8,
    /// Flag indicating if the pixel data is big-endian (1) or little-endian (0).
    pub big_endian_flag: u8,
    /// Flag indicating if the pixel format is true-colour (1) or colormapped (0).
    pub true_colour_flag: u8,
    /// Maximum red color value.
    pub red_max: u16,
    /// Maximum green color value.
    pub green_max: u16,
    /// Maximum blue color value.
    pub blue_max: u16,
    /// Number of shifts to apply to get the red color component.
    pub red_shift: u8,
    /// Number of shifts to apply to get the green color component.
    pub green_shift: u8,
    /// Number of shifts to apply to get the blue color component.
    pub blue_shift: u8,
}

impl PixelFormat {
    /// Creates a standard 32-bit RGBA pixel format.
    ///
    /// # Returns
    ///
    /// A `PixelFormat` instance configured for 32-bit RGBA.
    #[must_use]
    pub fn rgba32() -> Self {
        Self {
            bits_per_pixel: 32,
            depth: 24,
            big_endian_flag: 0,
            true_colour_flag: 1,
            red_max: 255,
            green_max: 255,
            blue_max: 255,
            red_shift: 0,
            green_shift: 8,
            blue_shift: 16,
        }
    }

    /// Checks if this `PixelFormat` is compatible with the standard 32-bit RGBA format.
    ///
    /// # Returns
    ///
    /// `true` if the pixel format matches 32-bit RGBA, `false` otherwise.
    #[must_use]
    pub fn is_compatible_with_rgba32(&self) -> bool {
        self.bits_per_pixel == 32
            && self.depth == 24
            && self.big_endian_flag == 0
            && self.true_colour_flag == 1
            && self.red_max == 255
            && self.green_max == 255
            && self.blue_max == 255
            && self.red_shift == 0
            && self.green_shift == 8
            && self.blue_shift == 16
    }

    /// Validates that this pixel format is supported.
    ///
    /// Checks that the format uses valid bits-per-pixel values and is either
    /// true-color or a supported color-mapped format.
    ///
    /// # Returns
    ///
    /// `true` if the format is valid and supported, `false` otherwise.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        // Check bits per pixel is valid
        if self.bits_per_pixel != 8
            && self.bits_per_pixel != 16
            && self.bits_per_pixel != 24
            && self.bits_per_pixel != 32
        {
            return false;
        }

        // Check depth is reasonable
        if self.depth == 0 || self.depth > 32 {
            return false;
        }

        // For non-truecolor (color-mapped), only 8bpp is supported
        if self.true_colour_flag == 0 && self.bits_per_pixel != 8 {
            return false;
        }

        // For truecolor, validate color component ranges
        if self.true_colour_flag != 0 {
            // Check that max values fit in the bit depth
            #[allow(clippy::cast_possible_truncation)]
            // leading_zeros() returns max 32, result always fits in u8
            let bits_needed = |max: u16| -> u8 {
                if max == 0 {
                    0
                } else {
                    (16 - max.leading_zeros()) as u8
                }
            };

            let red_bits = bits_needed(self.red_max);
            let green_bits = bits_needed(self.green_max);
            let blue_bits = bits_needed(self.blue_max);

            // Total bits should not exceed depth
            if red_bits + green_bits + blue_bits > self.depth {
                return false;
            }

            // Shifts should not cause overlap or exceed bit depth
            if self.red_shift >= 32 || self.green_shift >= 32 || self.blue_shift >= 32 {
                return false;
            }
        }

        true
    }

    /// Creates a 16-bit RGB565 pixel format.
    ///
    /// RGB565 uses 5 bits for red, 6 bits for green, and 5 bits for blue.
    /// This is a common format for embedded displays and bandwidth-constrained clients.
    ///
    /// # Returns
    ///
    /// A `PixelFormat` instance configured for 16-bit RGB565.
    #[must_use]
    pub fn rgb565() -> Self {
        Self {
            bits_per_pixel: 16,
            depth: 16,
            big_endian_flag: 0,
            true_colour_flag: 1,
            red_max: 31,   // 5 bits
            green_max: 63, // 6 bits
            blue_max: 31,  // 5 bits
            red_shift: 11,
            green_shift: 5,
            blue_shift: 0,
        }
    }

    /// Creates a 16-bit RGB555 pixel format.
    ///
    /// RGB555 uses 5 bits for each of red, green, and blue, with 1 unused bit.
    ///
    /// # Returns
    ///
    /// A `PixelFormat` instance configured for 16-bit RGB555.
    #[must_use]
    pub fn rgb555() -> Self {
        Self {
            bits_per_pixel: 16,
            depth: 15,
            big_endian_flag: 0,
            true_colour_flag: 1,
            red_max: 31,   // 5 bits
            green_max: 31, // 5 bits
            blue_max: 31,  // 5 bits
            red_shift: 10,
            green_shift: 5,
            blue_shift: 0,
        }
    }

    /// Creates an 8-bit BGR233 pixel format.
    ///
    /// BGR233 uses 2 bits for blue, 3 bits for green, and 3 bits for red.
    /// This format is used for very low bandwidth connections and legacy clients.
    ///
    /// # Returns
    ///
    /// A `PixelFormat` instance configured for 8-bit BGR233.
    #[must_use]
    pub fn bgr233() -> Self {
        Self {
            bits_per_pixel: 8,
            depth: 8,
            big_endian_flag: 0,
            true_colour_flag: 1,
            red_max: 7,   // 3 bits
            green_max: 7, // 3 bits
            blue_max: 3,  // 2 bits
            red_shift: 0,
            green_shift: 3,
            blue_shift: 6,
        }
    }

    /// Writes the pixel format data into a `BytesMut` buffer.
    ///
    /// This function serializes the `PixelFormat` into the RFB protocol format.
    ///
    /// # Arguments
    ///
    /// * `buf` - A mutable reference to the `BytesMut` buffer to write into.
    pub fn write_to(&self, buf: &mut BytesMut) {
        buf.put_u8(self.bits_per_pixel);
        buf.put_u8(self.depth);
        buf.put_u8(self.big_endian_flag);
        buf.put_u8(self.true_colour_flag);
        buf.put_u16(self.red_max);
        buf.put_u16(self.green_max);
        buf.put_u16(self.blue_max);
        buf.put_u8(self.red_shift);
        buf.put_u8(self.green_shift);
        buf.put_u8(self.blue_shift);
        buf.put_bytes(0, 3); // padding
    }

    /// Reads and deserializes a `PixelFormat` from a `BytesMut` buffer.
    ///
    /// This function extracts pixel format information from the RFB protocol stream.
    ///
    /// # Arguments
    ///
    /// * `buf` - A mutable reference to the `BytesMut` buffer to read from.
    ///
    /// # Returns
    ///
    /// `Ok(Self)` containing the parsed `PixelFormat`.
    ///
    /// # Errors
    ///
    /// Returns `Err(io::Error)` if there are not enough bytes in the buffer
    /// to read a complete `PixelFormat`.
    pub fn from_bytes(buf: &mut BytesMut) -> io::Result<Self> {
        if buf.len() < 16 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Not enough bytes for PixelFormat",
            ));
        }

        let pf = Self {
            bits_per_pixel: buf.get_u8(),
            depth: buf.get_u8(),
            big_endian_flag: buf.get_u8(),
            true_colour_flag: buf.get_u8(),
            red_max: buf.get_u16(),
            green_max: buf.get_u16(),
            blue_max: buf.get_u16(),
            red_shift: buf.get_u8(),
            green_shift: buf.get_u8(),
            blue_shift: buf.get_u8(),
        };
        buf.advance(3);
        Ok(pf)
    }
}

/// Trait defining the interface for RFB encoding implementations.
pub trait Encoding {
    /// Encodes raw pixel data into RFB-compatible byte stream.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw pixel data (RGBA format: 4 bytes per pixel)
    /// * `width` - Width of the framebuffer
    /// * `height` - Height of the framebuffer
    /// * `quality` - Quality level for lossy encodings (0-100)
    /// * `compression` - Compression level (0-9)
    ///
    /// # Returns
    ///
    /// Encoded data as `BytesMut`
    fn encode(
        &self,
        data: &[u8],
        width: u16,
        height: u16,
        quality: u8,
        compression: u8,
    ) -> BytesMut;
}

/// Creates an encoder instance for the specified encoding type.
///
/// # Arguments
///
/// * `encoding_type` - The RFB encoding type constant
///
/// # Returns
///
/// `Some(Box<dyn Encoding>)` if the encoding is supported, `None` otherwise
#[must_use]
pub fn get_encoder(encoding_type: i32) -> Option<Box<dyn Encoding>> {
    match encoding_type {
        ENCODING_RAW => Some(Box::new(RawEncoding)),
        ENCODING_RRE => Some(Box::new(RreEncoding)),
        ENCODING_CORRE => Some(Box::new(CorRreEncoding)),
        ENCODING_HEXTILE => Some(Box::new(HextileEncoding)),
        ENCODING_TIGHT => Some(Box::new(TightEncoding)),
        ENCODING_TIGHTPNG => Some(Box::new(TightPngEncoding)),
        _ => None,
    }
}
